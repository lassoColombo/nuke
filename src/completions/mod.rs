use crate::client::config_from_context;
use crate::discovery::DiscoveryCache;
use anyhow::Result;
use itertools::Itertools;
use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::{Api, DynamicObject, ListParams},
    Client, ResourceExt,
};
use nu_protocol::ast::Expr;

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
            ..Default::default()
        })
        .collect())
}

pub async fn complete_api_group(
    context: Option<String>,
) -> Result<Vec<nu_protocol::DynamicSuggestion>> {
    let config = config_from_context(context).await?;
    let client = Client::try_from(config.clone())?;
    let cache = DiscoveryCache::load(&client, &config).await?;
    Ok(cache
        .entries()
        .map(|entry| entry.group.as_str())
        .unique()
        .map(|g| nu_protocol::DynamicSuggestion {
            value: g.to_string(),
            ..Default::default()
        })
        .collect())
}

pub async fn complete_namespaces(
    context: Option<String>,
) -> Result<Vec<nu_protocol::DynamicSuggestion>> {
    let config = config_from_context(context).await?;
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
            ..Default::default()
        })
        .collect())
}

// ---------------------------------------------------------------------------
// AST helpers for dynamic completions
// ---------------------------------------------------------------------------

pub fn expr_as_str(expr: &nu_protocol::ast::Expression) -> Option<&str> {
    match &expr.expr {
        Expr::String(s) => Some(s.as_str()),
        Expr::GlobPattern(s, _) => Some(s.as_str()),
        _ => None,
    }
}

pub fn flag_str<'a>(call: &'a nu_protocol::ast::Call, name: &str) -> Option<&'a str> {
    call.named_iter()
        .find(|(n, _, _)| n.item == name)
        .and_then(|(_, _, expr)| expr.as_ref())
        .and_then(|e| expr_as_str(e))
}

// ---------------------------------------------------------------------------
// Output format
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Compact,
    Wide,
    Full,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "compact" => Some(Self::Compact),
            "wide" => Some(Self::Wide),
            "full" => Some(Self::Full),
            _ => None,
        }
    }
}

pub fn complete_output() -> Vec<nu_protocol::DynamicSuggestion> {
    vec![
        nu_protocol::DynamicSuggestion {
            value: "compact".into(),
            description: Some("Compact single-line record".into()),
            ..Default::default()
        },
        nu_protocol::DynamicSuggestion {
            value: "wide".into(),
            description: Some("Wide record with extra columns".into()),
            ..Default::default()
        },
        nu_protocol::DynamicSuggestion {
            value: "full".into(),
            description: Some("Raw resource value tree".into()),
            ..Default::default()
        },
    ]
}
