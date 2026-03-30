use anyhow::Result;
use futures::future::join_all;
use kube::{
    api::{Api, DynamicObject, ListParams},
    Client,
};
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};

use crate::completions::{complete_clusters, complete_users, expr_as_str};
use crate::completions::{
    complete_contexts, complete_namespaces, complete_resource_instances, complete_resource_names,
    flag_str,
};
use crate::conversions::dynamic_object_to_raw_value;
use crate::discovery::DiscoveryCache;
use crate::formatters::OutputFormat;
use crate::{completions::complete_output, plugin::NukePlugin};

pub struct GetCommand;

// ---------------------------------------------------------------------------
// PluginCommand impl
// ---------------------------------------------------------------------------

impl PluginCommand for GetCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke get"
    }

    fn description(&self) -> &str {
        "Get Kubernetes resources"
    }

    fn signature(&self) -> Signature {
        Signature::build("nuke get")
            .named("user", SyntaxShape::String, "Kubeconfig user to use", None)
            .named(
                "context",
                SyntaxShape::String,
                "Kubeconfig context to use",
                None,
            )
            .named(
                "cluster",
                SyntaxShape::String,
                "Kubeconfig cluster to use",
                None,
            )
            .required(
                "resource",
                SyntaxShape::String,
                "Resource type: name/short-name (pods, po), category (all), or fully-qualified group/version/plural (metrics.k8s.io/v1beta1/pods, v1/pods)",
            )
            .optional(
                "name",
                SyntaxShape::String,
                "Resource name (omit to list all)",
            )
            .named(
                "namespace",
                SyntaxShape::String,
                "Namespace to use",
                Some('n'),
            )
            .named(
                "output",
                SyntaxShape::String,
                "Output format: compact | wide | full  (default: compact for lists, wide for single)",
                Some('o'),
            )
            .switch(
                "all-namespaces",
                "List resources across all namespaces",
                Some('A'),
            )
            .input_output_types(vec![
                (Type::Nothing, Type::Table(vec![].into())),
            ])
            .category(Category::Custom("kubernetes".to_string()))
    }

    fn run(
        &self,
        plugin: &NukePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        plugin
            .rt
            .block_on(run_get(plugin, call))
            .map_err(|e| LabeledError::new(e.to_string()))
    }

    fn get_dynamic_completion(
        &self,
        plugin: &NukePlugin,
        _engine: &EngineInterface,
        call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        let context = flag_str(&call.call, "context").map(|s| s.to_string());
        let cluster = flag_str(&call.call, "cluster").map(|s| s.to_string());
        let user = flag_str(&call.call, "user").map(|s| s.to_string());
        match arg_type {
            ArgType::Positional(0) => Some(
                plugin
                    .rt
                    .block_on(complete_resource_names(context, cluster, user))
                    .unwrap_or_default(),
            ),
            ArgType::Positional(1) => {
                let resource = call
                    .call
                    .positional_nth(0)
                    .and_then(|e| expr_as_str(e))
                    .map(|s| s.to_string())?;

                let namespace = flag_str(&call.call, "namespace").map(|s| s.to_string());

                let suggestions = plugin.rt.block_on(complete_resource_instances(
                    &resource,
                    namespace.as_deref(),
                    context,
                    cluster,
                    user,
                ));
                Some(suggestions.unwrap_or_default())
            }

            ArgType::Flag(ref name) => match name.as_ref() {
                "namespace" => Some(
                    plugin
                        .rt
                        .block_on(complete_namespaces(context, cluster, user))
                        .unwrap_or_default(),
                ),
                "context" => Some(complete_contexts()),
                "cluster" => Some(complete_clusters()),
                "user" => Some(complete_users()),
                "output" => Some(complete_output()),
                _ => None,
            },

            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Async run
// ---------------------------------------------------------------------------

async fn run_get(plugin: &NukePlugin, call: &EvaluatedCall) -> Result<PipelineData> {
    let resource: String = call.req(0)?;
    let name: Option<String> = call.opt(1)?;
    let namespace_flag: Option<String> = call.get_flag("namespace")?;
    let all_namespaces: bool = call.has_flag("all-namespaces")?;
    let output_flag: Option<String> = call.get_flag("output")?;

    let explicit_format = output_flag.as_deref().and_then(OutputFormat::from_str);

    let config = kube::Config::from_kubeconfig(&kube::config::KubeConfigOptions {
        context: call.get_flag("context")?,
        cluster: call.get_flag("cluster")?,
        user: call.get_flag("user")?,
    })
    .await?;
    let default_ns = config.default_namespace.clone();
    let client = Client::try_from(config.clone())?;

    let namespace = namespace_flag.as_deref().unwrap_or(&default_ns).to_string();

    let cache = DiscoveryCache::load(&client, &config).await?;

    // ── Fully-qualified resource lookup  e.g. metrics.k8s.io/v1beta1/pods ──
    if let Some((group, version, plural)) = parse_fqn(&resource) {
        let entry = cache
            .find_by_gvr(&group, &version, &plural)
            .ok_or_else(|| {
                anyhow::anyhow!("no resource '{plural}' in group '{group}' version '{version}'")
            })?;
        return run_get_single(
            plugin,
            call,
            client,
            entry,
            name,
            namespace,
            all_namespaces,
            explicit_format,
        )
        .await;
    }

    if let Some(entry) = cache.find(&resource) {
        // ── Single resource type ──────────────────────────────────────────────
        run_get_single(
            plugin,
            call,
            client,
            entry,
            name,
            namespace,
            all_namespaces,
            explicit_format,
        )
        .await
    } else {
        // ── Category fallback (e.g. "all", "api-extensions") ──────────────────
        let category_entries: Vec<crate::discovery::ResourceEntry> = cache
            .find_by_category(&resource)
            .into_iter()
            .cloned()
            .collect();

        if category_entries.is_empty() {
            return Err(anyhow::anyhow!(
                "unknown resource type or category: '{}'",
                resource
            ));
        }
        if name.is_some() {
            return Err(anyhow::anyhow!(
                "cannot specify a resource name when querying by category '{}'",
                resource
            ));
        }

        run_get_category(
            plugin,
            call,
            client,
            category_entries,
            namespace,
            all_namespaces,
            explicit_format,
        )
        .await
    }
}

/// Parse a fully-qualified resource specifier into (group, version, plural).
///
/// Accepted forms:
///   - `version/plural`           → group = "" (core)   e.g. "v1/pods"
///   - `group/version/plural`     → named group          e.g. "apps/v1/deployments"
///                                                        e.g. "metrics.k8s.io/v1beta1/pods"
///
/// Returns `None` when the string contains no `/` (plain name or category).
fn parse_fqn(s: &str) -> Option<(String, String, String)> {
    let slash_count = s.chars().filter(|&c| c == '/').count();
    match slash_count {
        0 => None,
        1 => {
            let (version, plural) = s.split_once('/')?;
            Some((String::new(), version.to_string(), plural.to_string()))
        }
        _ => {
            // Split on the first slash for the group, then the second for version/plural.
            let (group, rest) = s.split_once('/')?;
            let (version, plural) = rest.split_once('/')?;
            // Anything after a third slash is invalid; plural must not contain '/'.
            if plural.contains('/') {
                return None;
            }
            Some((group.to_string(), version.to_string(), plural.to_string()))
        }
    }
}

async fn run_get_single(
    plugin: &NukePlugin,
    call: &EvaluatedCall,
    client: Client,
    entry: &crate::discovery::ResourceEntry,
    name: Option<String>,
    namespace: String,
    all_namespaces: bool,
    explicit_format: Option<OutputFormat>,
) -> Result<PipelineData> {
    let ar = entry_to_ar(entry);

    let api: Api<DynamicObject> = if all_namespaces || !entry.namespaced {
        Api::all_with(client, &ar)
    } else {
        Api::namespaced_with(client, &namespace, &ar)
    };

    let list = match name {
        Some(ref n) => vec![api.get(n).await?],
        _ => api.list(&ListParams::default()).await?.items,
    };

    let span = call.head;
    let format = explicit_format.unwrap_or_else(|| {
        if name.is_some() {
            OutputFormat::Wide
        } else {
            OutputFormat::Compact
        }
    });

    if format == OutputFormat::Full {
        let rows: Vec<Value> = list
            .iter()
            .map(|item| dynamic_object_to_raw_value(item, span))
            .collect();
        let result = match rows.as_slice() {
            [single] => single.clone(),
            _ => Value::list(rows, span),
        };
        return Ok(PipelineData::Value(result, None));
    }

    let formatter = plugin
        .formatter_registry
        .get(&entry.group, &entry.version, &entry.plural);

    let rows: Vec<Value> = list
        .iter()
        .map(|item| formatter.format(item, span, format))
        .collect();

    let result = match rows.as_slice() {
        [single] if name.is_some() => single.clone(),
        _ => Value::list(rows, span),
    };
    Ok(PipelineData::Value(result, None))
}

async fn run_get_category(
    plugin: &NukePlugin,
    call: &EvaluatedCall,
    client: Client,
    entries: Vec<crate::discovery::ResourceEntry>,
    namespace: String,
    all_namespaces: bool,
    explicit_format: Option<OutputFormat>,
) -> Result<PipelineData> {
    let span = call.head;
    let format = explicit_format.unwrap_or(OutputFormat::Compact);

    // Fetch all resource types concurrently.
    let futures: Vec<_> = entries
        .iter()
        .map(|entry| {
            let client = client.clone();
            let namespace = namespace.clone();
            let ar = entry_to_ar(entry);
            let namespaced = entry.namespaced;
            async move {
                let api: Api<DynamicObject> = if all_namespaces || !namespaced {
                    Api::all_with(client, &ar)
                } else {
                    Api::namespaced_with(client, &namespace, &ar)
                };
                api.list(&ListParams::default()).await.map(|l| l.items)
            }
        })
        .collect();

    let results = join_all(futures).await;

    let mut cols: Vec<String> = Vec::new();
    let mut vals: Vec<Value> = Vec::new();
    for (entry, result) in entries.iter().zip(results) {
        let items = match result {
            Ok(items) => items,
            // Skip resource types that can't be listed (e.g. lack LIST verb).
            Err(_) => continue,
        };

        let rows: Vec<Value> = if format == OutputFormat::Full {
            items
                .iter()
                .map(|item| dynamic_object_to_raw_value(item, span))
                .collect()
        } else {
            let formatter =
                plugin
                    .formatter_registry
                    .get(&entry.group, &entry.version, &entry.plural);
            items
                .iter()
                .map(|item| formatter.format(item, span, format))
                .collect()
        };

        cols.push(entry.plural.clone());
        vals.push(Value::list(rows, span));
    }

    Ok(PipelineData::Value(
        Value::record(
            nu_protocol::Record::from_raw_cols_vals(cols, vals, span, span).unwrap_or_default(),
            span,
        ),
        None,
    ))
}

fn entry_to_ar(entry: &crate::discovery::ResourceEntry) -> kube::discovery::ApiResource {
    kube::discovery::ApiResource {
        group: entry.group.clone(),
        version: entry.version.clone(),
        kind: entry.kind.clone(),
        plural: entry.plural.clone(),
        api_version: if entry.group.is_empty() {
            entry.version.clone()
        } else {
            format!("{}/{}", entry.group, entry.version)
        },
    }
}
