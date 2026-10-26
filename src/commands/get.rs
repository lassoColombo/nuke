//! `nuke get`: list or fetch any resource the cluster serves, by any name
//! kubectl accepts.

use anyhow::Result;
use futures::future::join_all;
use kube::{
    Client,
    api::{DynamicObject, ListParams},
};
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
// nu-plugin marks the dynamic-completion API experimental (deprecated) as a
// whole; implementing it is the only way to offer completions.
#[allow(deprecated)]
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{
    Category, DynamicSuggestion, LabeledError, PipelineData, Record, Signature, Span, SyntaxShape,
    Type, Value,
};

use crate::completions::{
    complete_common_flag, complete_labels, complete_resource_instances, complete_resource_names,
    flag_prefix, flag_str, label_filter, positional_str, selection,
};
use crate::conversions::dynamic_object_to_raw_value;
use crate::decorators::{self, Decorator};
use crate::discovery::ResourceEntry;
use crate::formatters::{OutputFormat, formatter_for};
use crate::kube_env::{self, KubeEnv, cluster_flags};
use crate::plugin::NukePlugin;

pub struct GetCommand;

impl PluginCommand for GetCommand {
    type Plugin = NukePlugin;

    fn name(&self) -> &str {
        "nuke get"
    }

    fn description(&self) -> &str {
        "Get Kubernetes resources"
    }

    fn signature(&self) -> Signature {
        let sig = cluster_flags(Signature::build("nuke get"))
            .required(
                "resource",
                SyntaxShape::String,
                "Resource type: name/short-name (pods, po), name.group (deploy.apps), category (all), or fully-qualified group/version/plural (metrics.k8s.io/v1beta1/pods, v1/pods)",
            )
            .optional("name", SyntaxShape::String, "Resource name (omit to list all)")
            .named("namespace", SyntaxShape::String, "Namespace to use", Some('n'))
            .named(
                "output",
                SyntaxShape::String,
                "Output format: compact | wide | full  (default: compact for lists, wide for single)",
                Some('o'),
            )
            .switch("all-namespaces", "List resources across all namespaces", Some('A'))
            .named(
                "labels",
                SyntaxShape::String,
                "Label selector: comma-separated key=value pairs (e.g. app=web,tier=frontend)",
                Some('l'),
            );
        decorators::switches(sig)
            // A list is a table; a get by name, and a category (one list per
            // type), are a record.
            .input_output_types(vec![
                (Type::Nothing, Type::Table(vec![].into())),
                (Type::Nothing, Type::Record(vec![].into())),
            ])
            .category(Category::Custom("kubernetes".to_string()))
    }

    fn run(
        &self,
        plugin: &NukePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let env = KubeEnv::from_engine(engine);
        plugin
            .rt
            .block_on(run_get(plugin, &env, call))
            .map_err(crate::commands::to_labeled)
    }

    #[allow(deprecated)]
    fn get_dynamic_completion(
        &self,
        plugin: &NukePlugin,
        engine: &EngineInterface,
        call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<DynamicSuggestion>> {
        let env = KubeEnv::from_engine(engine);
        let sel = selection(&call.call);
        let namespace = flag_str(&call.call, "namespace");
        let result = match arg_type {
            ArgType::Positional(0) => plugin
                .rt
                .block_on(complete_resource_names(plugin, &env, &sel)),
            ArgType::Positional(1) => {
                let resource = positional_str(&call.call, 0)?;
                plugin.rt.block_on(complete_resource_instances(
                    plugin, &env, resource, namespace, &sel,
                ))
            }
            ArgType::Flag(ref name) if matches!(name.as_ref(), "labels" | "l") => {
                let resource = positional_str(&call.call, 0)?;
                let all_namespaces = call
                    .call
                    .named_iter()
                    .any(|(n, _, _)| n.item == "all-namespaces" || n.item == "A");
                let labels = plugin
                    .rt
                    .block_on(complete_labels(
                        plugin,
                        &env,
                        resource,
                        namespace,
                        all_namespaces,
                        &sel,
                    ))
                    .ok()?;
                return Some(label_filter(flag_prefix(&call, "labels"), &labels));
            }
            ArgType::Flag(ref name) => return complete_common_flag(plugin, &env, &call, name),
            ArgType::Positional(_) => return None,
        };
        Some(result.unwrap_or_default())
    }
}

/// What one `nuke get` call asked for, once the flags are parsed.
struct Query {
    name: Option<String>,
    /// The namespace to read, `None` for `--all-namespaces`.
    namespace: Option<String>,
    /// The label selector, passed to the apiserver verbatim.
    params: ListParams,
    format: Option<OutputFormat>,
    decorators: Vec<&'static Decorator>,
    span: Span,
}

async fn run_get(plugin: &NukePlugin, env: &KubeEnv, call: &EvaluatedCall) -> Result<PipelineData> {
    let resource: String = call.req(0)?;
    let name: Option<String> = call.opt(1)?;
    let all_namespaces = call.has_flag("all-namespaces")?;
    if name.is_some() && all_namespaces {
        anyhow::bail!("a resource cannot be retrieved by name across all namespaces");
    }

    let selection = kube_env::selection(call)?;
    let (config, client) = env.connect(&selection).await?;
    let namespace = match all_namespaces {
        true => None,
        false => Some(
            call.get_flag("namespace")?
                .unwrap_or(config.default_namespace.clone()),
        ),
    };
    // Trim to tolerate stray whitespace; an empty selector selects everything.
    let params = match call.get_flag::<String>("labels")? {
        Some(sel) if !sel.trim().is_empty() => {
            if name.is_some() {
                anyhow::bail!("a label selector cannot be combined with a resource name");
            }
            ListParams::default().labels(sel.trim())
        }
        _ => ListParams::default(),
    };
    let query = Query {
        name,
        namespace,
        params,
        format: OutputFormat::from_flag(call)?,
        decorators: decorators::active(call)?,
        span: call.head,
    };

    let cache = plugin.discovery_or_populate(env, &selection, &config)?;
    // A category first, as kubectl's `replaceAliases` expands one before
    // resolving any resource name.
    let category: Vec<&ResourceEntry> = cache
        .find_by_category(&resource)
        .into_iter()
        .filter(|e| e.verbs.iter().any(|v| v == "list"))
        .collect();
    let value = match (category.is_empty(), cache.resolve(&resource)) {
        (false, _) if query.name.is_some() => {
            anyhow::bail!("cannot specify a resource name when querying by category '{resource}'")
        }
        (false, _) => get_category(client, &category, &query).await?,
        (true, Some(entry)) => get_one_type(client, entry, &query).await?,
        (true, None) => anyhow::bail!("unknown resource type or category: '{resource}'"),
    };
    Ok(PipelineData::Value(value, None))
}

/// One resource type: a list, or the record itself for a get by name.
async fn get_one_type(client: Client, entry: &ResourceEntry, q: &Query) -> Result<Value> {
    let api = entry.api(client, q.namespace.as_deref());
    let items = match &q.name {
        Some(name) => vec![api.get(name).await?],
        None => api.list(&q.params).await?.items,
    };
    let format = q
        .format
        .unwrap_or(OutputFormat::default_for(q.name.is_some()));
    let mut rows = rows(entry, &items, format, q);
    // A get by name is the one record; a list stays a list, even when it
    // happens to hold one row.
    Ok(match (q.name.is_some(), rows.pop()) {
        (true, Some(single)) => single,
        (_, popped) => {
            rows.extend(popped);
            Value::list(rows, q.span)
        }
    })
}

/// Every type of a category, listed concurrently, as a record of
/// `plural: rows` — a category's rows have different shapes, so it cannot be
/// one table.
async fn get_category(client: Client, entries: &[&ResourceEntry], q: &Query) -> Result<Value> {
    let format = q.format.unwrap_or(OutputFormat::Compact);
    let lists = join_all(entries.iter().map(|entry| {
        let api = entry.api(client.clone(), q.namespace.as_deref());
        async move { api.list(&q.params).await.map(|l| l.items) }
    }))
    .await;

    let mut record = Record::new();
    for (entry, result) in entries.iter().zip(lists) {
        let items = match result {
            Ok(items) => items,
            // Gone since discovery, or not listable after all: nothing to show.
            Err(kube::Error::Api(status)) if matches!(status.code, 404 | 405) => continue,
            // kubectl prints the error and carries on with the other types.
            Err(kube::Error::Api(status)) if status.code == 403 => {
                eprintln!("nuke: skipping {}: {}", entry.plural, status.message);
                continue;
            }
            Err(e) => {
                return Err(anyhow::Error::new(e).context(format!("listing {}", entry.plural)));
            }
        };
        // Two groups may put the same plural in one category: those keys
        // carry their group, as kubectl's `plural.group`.
        let shared = entries.iter().filter(|e| e.plural == entry.plural).count() > 1;
        let key = match shared {
            true => format!("{}.{}", entry.plural, entry.group),
            false => entry.plural.clone(),
        };
        record.push(key, Value::list(rows(entry, &items, format, q), q.span));
    }
    Ok(Value::record(record, q.span))
}

/// `items` of one resource type rendered in `format`: raw objects for
/// `full`, otherwise formatted and decorated records.
fn rows(
    entry: &ResourceEntry,
    items: &[DynamicObject],
    format: OutputFormat,
    q: &Query,
) -> Vec<Value> {
    let span = q.span;
    if format == OutputFormat::Full {
        let ar = entry.to_api_resource();
        return items
            .iter()
            .map(|item| dynamic_object_to_raw_value(item, &ar, span))
            .collect();
    }
    let formatter = formatter_for(&entry.group, &entry.version, &entry.plural);
    let wide = format == OutputFormat::Wide;
    items
        .iter()
        .map(|item| {
            let mut rec = formatter.record(item, span, wide);
            decorators::apply(&mut rec, item, &q.decorators, span);
            Value::record(rec, span)
        })
        .collect()
}
