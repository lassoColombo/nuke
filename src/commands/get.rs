#[allow(deprecated)]
use anyhow::Result;
use kube::{
    Client, Config, ResourceExt,
    api::{Api, DynamicObject, ListParams},
};
use k8s_openapi::api::core::v1::Namespace;
use nu_plugin::{PluginCommand, EngineInterface, EvaluatedCall, DynamicCompletionCall};
use nu_protocol::{
    Category, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value,
};
use nu_protocol::engine::{ArgType, ExperimentalMarker};

use crate::plugin::KubectlPlugin;
use crate::discovery::DiscoveryCache;

pub struct GetCommand;

#[allow(deprecated)]
impl PluginCommand for GetCommand {
    type Plugin = KubectlPlugin;

    fn name(&self) -> &str {
        "kube get"
    }

    fn description(&self) -> &str {
        "Get Kubernetes resources"
    }

    fn signature(&self) -> Signature {
        Signature::build("kube get")
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
                "context",
                SyntaxShape::String,
                "Kubeconfig context to use",
                None,
            )
            .named(
                "output",
                SyntaxShape::String,
                "Output format: json, yaml, wide, name",
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
        plugin: &KubectlPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        plugin.rt.block_on(run_get(call))
            .map_err(|e| LabeledError::new(e.to_string()))
    }

    fn get_dynamic_completion(
        &self,
        plugin: &KubectlPlugin,
        _engine: &EngineInterface,
        _call: DynamicCompletionCall,
        arg_type: ArgType<'_>,
        _experimental: ExperimentalMarker,
    ) -> Option<Vec<nu_protocol::DynamicSuggestion>> {
        match arg_type {
            ArgType::Positional(0) => {
                Some(complete_resource_names())
            }
            ArgType::Flag(ref name) => {
                match name.as_ref() {
                    "namespace" => {
                        let suggestions = plugin.rt.block_on(complete_namespaces());
                        Some(suggestions.unwrap_or_default())
                    }
                    "context" => Some(complete_contexts()),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Async implementation
// ---------------------------------------------------------------------------

async fn run_get(call: &EvaluatedCall) -> Result<PipelineData> {
    let resource: String = call.req(0)?;
    let name: Option<String> = call.opt(1)?;
    let namespace_flag: Option<String> = call.get_flag("namespace")?;
    let context_flag:   Option<String> = call.get_flag("context")?;
    let all_namespaces: bool           = call.has_flag("all-namespaces")?;

    let config = build_config(context_flag).await?;
    let default_ns = config.default_namespace.clone();
    let client = Client::try_from(config.clone())?;

    let namespace = namespace_flag
        .as_deref()
        .unwrap_or(&default_ns)
        .to_string();

    let cache = DiscoveryCache::load(&client, &config).await?;

    let entry = cache
        .find(&resource)
        .ok_or_else(|| anyhow::anyhow!("unknown resource type: '{}'", resource))?;

    let ar = kube::discovery::ApiResource {
        group:       entry.group.clone(),
        version:     entry.version.clone(),
        kind:        entry.kind.clone(),
        plural:      entry.plural.clone(),
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

    let span = call.head;
    let rows: Vec<Value> = list
        .iter()
        .map(|item| resource_to_value(item, span))
        .collect();

    Ok(PipelineData::Value(Value::list(rows, span), None))
}

async fn build_config(context: Option<String>) -> Result<Config> {
    if let Some(ctx) = context {
        let options = kube::config::KubeConfigOptions {
            context: Some(ctx),
            cluster: None,
            user:    None,
        };
        Ok(Config::from_kubeconfig(&options).await?)
    } else {
        Ok(Config::infer().await?)
    }
}

fn resource_to_value(item: &DynamicObject, span: nu_protocol::Span) -> Value {
    let name      = item.name_any();
    let namespace = item.namespace().unwrap_or_default();
    let age       = item
        .creation_timestamp()
        .map(|t| t.0.to_string())
        .unwrap_or_default();

    let mut record = nu_protocol::Record::new();
    record.push("name",      Value::string(name,      span));
    record.push("namespace", Value::string(namespace, span));
    record.push("age",       Value::string(age,       span));

    Value::record(record, span)
}

// ---------------------------------------------------------------------------
// Completers
// ---------------------------------------------------------------------------

fn complete_resource_names() -> Vec<nu_protocol::DynamicSuggestion> {
    let kubeconfig = match kube::config::Kubeconfig::read() {
        Ok(k) => k,
        Err(_) => return vec![],
    };

    let context_name = kubeconfig.current_context.clone().unwrap_or_default();
    let context = kubeconfig
        .contexts
        .iter()
        .find(|c| c.name == context_name)
        .and_then(|c| c.context.as_ref());

    let cluster_name: String = context
        .map(|c| c.cluster.clone())
        .unwrap_or_default();

    let server = kubeconfig
        .clusters
        .iter()
        .find(|c| c.name == cluster_name)
        .and_then(|c| c.cluster.as_ref())
        .and_then(|c| c.server.as_deref())
        .unwrap_or_default()
        .to_string();

    let host_dir = server_to_cache_dir_name(&server);
    let cache_dir = match dirs::home_dir() {
        Some(h) => h.join(".kube").join("cache").join("discovery").join(host_dir),
        None => return vec![],
    };

    read_names_from_cache(&cache_dir)
        .into_iter()
        .map(|(name, description)| nu_protocol::DynamicSuggestion {
            value: name,
            description: Some(description),
            ..Default::default()
        })
        .collect()
}

fn server_to_cache_dir_name(server: &str) -> String {
    server
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .replace(':', "_")
}

fn read_names_from_cache(cache_dir: &std::path::Path) -> Vec<(String, String)> {
    let mut names = Vec::new();

    let walker = match std::fs::read_dir(cache_dir) {
        Ok(w) => w,
        Err(_) => return names,
    };

    for entry in walker.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Ok(subentries) = std::fs::read_dir(&path) {
                for subentry in subentries.flatten() {
                    let subpath = subentry.path();
                    if subpath.is_dir() {
                        let json = subpath.join("serverresources.json");
                        extract_names_from_file(&json, &mut names);
                    }
                }
            }
        }
    }

    names
}

fn extract_names_from_file(
    path: &std::path::Path,
    names: &mut Vec<(String, String)>,
) {
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResourceList;

    let Ok(content) = std::fs::read_to_string(path) else { return };
    let Ok(list) = serde_json::from_str::<APIResourceList>(&content) else { return };

    for resource in &list.resources {
        if resource.name.contains('/') { continue; }

        let kind = resource.kind.clone();

        names.push((resource.name.clone(), kind.clone()));

        if !resource.singular_name.is_empty() {
            names.push((resource.singular_name.clone(), kind.clone()));
        }

        if let Some(ref shorts) = resource.short_names {
            for short in shorts {
                names.push((short.clone(), kind.clone()));
            }
        }
    }
}

async fn complete_namespaces() -> Result<Vec<nu_protocol::DynamicSuggestion>> {
    let config = Config::infer().await?;
    let client = Client::try_from(config)?;

    let ns_api: Api<Namespace> = Api::all(client);
    let list = ns_api.list(&ListParams::default()).await?;

    Ok(list
        .items
        .iter()
        .map(|ns| nu_protocol::DynamicSuggestion {
            value: ns.name_any(),
            description: None,
            ..Default::default()
        })
        .collect())
}

fn complete_contexts() -> Vec<nu_protocol::DynamicSuggestion> {
    let Ok(kubeconfig) = kube::config::Kubeconfig::read() else {
        return vec![];
    };

    kubeconfig
        .contexts
        .iter()
        .map(|c| nu_protocol::DynamicSuggestion {
            value: c.name.clone(),
            description: None,
            ..Default::default()
        })
        .collect()
}
