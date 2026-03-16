use anyhow::Result;
use kube::{
    Client, Config, ResourceExt,
    api::{Api, ListParams, DynamicObject},
};
use k8s_openapi::api::core::v1::Namespace;
use crate::client::config_from_context;
use crate::discovery::DiscoveryCache;

// ----------
//  public   
// ----------

pub async fn complete_resource_names(
    context: Option<String>,
) -> Result<Vec<nu_protocol::DynamicSuggestion>> {
    let config = config_from_context(context).await?;
    let client = Client::try_from(config.clone())?;

    let cache = DiscoveryCache::load(&client, &config).await?;

    Ok(cache
        .entries()
        .map(|entry| nu_protocol::DynamicSuggestion {
            value: entry.plural.clone(),
            description: Some(entry.kind.clone()),
            ..Default::default()
        })
        .collect())
}

pub async fn complete_namespaces() -> Result<Vec<nu_protocol::DynamicSuggestion>> {
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

pub fn complete_contexts() -> Vec<nu_protocol::DynamicSuggestion> {
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

/// Complete instance names for a given resource type.
/// e.g. "pods" -> ["coredns-abc-123", "kube-proxy-xyz", ...]
pub async fn complete_resource_instances(
    resource: &str,
    namespace: Option<&str>,
    context: Option<String>,
) -> Result<Vec<nu_protocol::DynamicSuggestion>> {
    let config = config_from_context(context).await?;
    let default_ns = config.default_namespace.clone();
    let client = Client::try_from(config.clone())?;

    let cache = DiscoveryCache::load(&client, &config).await?;

    let entry = cache
        .find(resource)
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

    let ns = namespace.unwrap_or(&default_ns);

    let api: Api<DynamicObject> = if entry.namespaced {
        Api::namespaced_with(client, ns, &ar)
    } else {
        Api::all_with(client, &ar)
    };

    let list = api.list(&ListParams::default()).await?;

    Ok(list
        .items
        .iter()
        .map(|item| nu_protocol::DynamicSuggestion {
            value: item.name_any(),
            description: item.namespace().map(|ns| format!("namespace: {ns}")),
            ..Default::default()
        })
        .collect())
}
