use anyhow::Result;
use kube::{
    api::{Api, DynamicObject, ListParams},
    Client,
};
use nu_plugin::{DynamicCompletionCall, EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::engine::{ArgType, ExperimentalMarker};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value};

use crate::completions::expr_as_str;
use crate::completions::{
    complete_contexts, complete_namespaces, complete_resource_instances, complete_resource_names,
    flag_str,
};
use crate::discovery::DiscoveryCache;
use crate::formatters::OutputFormat;
use crate::types::dynamic_object_to_raw_value;
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
                "Resource type (pods, nodes, deployments…)",
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
                "Output format: compact | wide | full  (default: wide for lists, compact for single)",
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

    // Resolve the requested output format, or defer to the default which
    // depends on whether we are listing or fetching a single resource.
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

    let entry = cache
        .find(&resource)
        .ok_or_else(|| anyhow::anyhow!("unknown resource type: '{}'", resource))?;

    let ar = kube::discovery::ApiResource {
        group: entry.group.clone(),
        version: entry.version.clone(),
        kind: entry.kind.clone(),
        plural: entry.plural.clone(),
        api_version: if entry.group.is_empty() {
            entry.version.clone()
        } else {
            format!("{}/{}", entry.group, entry.version)
        },
    };

    let api: Api<DynamicObject> = if all_namespaces || !entry.namespaced {
        Api::all_with(client.clone(), &ar)
    } else {
        Api::namespaced_with(client.clone(), &namespace, &ar)
    };

    let list = if let Some(ref n) = name {
        let item = api.get(n).await?;
        vec![item]
    } else {
        api.list(&ListParams::default()).await?.items
    };

    let is_single = name.is_some();
    let span = call.head;

    let format = explicit_format.unwrap_or_else(|| {
        if is_single {
            OutputFormat::Wide
        } else {
            OutputFormat::Compact
        }
    });

    // Full format: skip all formatters, return raw nushell Value tree.
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

    // Formatter dispatch via registry stored on the plugin.
    let formatter = plugin
        .formatter_registry
        .get(&entry.group, &entry.version, &entry.plural);

    let rows: Vec<Value> = list
        .iter()
        .map(|item| formatter.format(item, span, format))
        .collect();

    Ok(PipelineData::Value(Value::list(rows, span), None))
}
