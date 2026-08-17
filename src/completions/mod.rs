use crate::plugin::NukePlugin;
use anyhow::Result;
use itertools::Itertools;
use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::{Api, DynamicObject, ListParams},
    Client, ResourceExt,
};
use nu_plugin::DynamicCompletionCall;
use nu_protocol::ast::Expr;
use nu_protocol::DynamicSuggestion;
use std::collections::{BTreeMap, BTreeSet};

fn format_group_version(group: &str, version: &str) -> String {
    let group = if group.is_empty() { "core" } else { group };
    format!("{}/{}", group, version)
}

// ----------
//  public
// ----------

pub async fn complete_resource_names(
    plugin: &NukePlugin,
    context: Option<String>,
    cluster: Option<String>,
    user: Option<String>,
) -> Result<Vec<nu_protocol::DynamicSuggestion>> {
    let config = kube::Config::from_kubeconfig(&kube::config::KubeConfigOptions {
        context,
        cluster,
        user,
    })
    .await?;
    let cache = plugin.discovery(&config)?;

    let mut suggestions: Vec<nu_protocol::DynamicSuggestion> = cache
        .entries()
        .map(|entry| nu_protocol::DynamicSuggestion {
            value: entry.plural.clone(),
            description: Some(format_group_version(&entry.group, &entry.version)),
            ..Default::default()
        })
        .collect();

    // Kind-only resources (e.g. PodMetrics) are reachable only by their kind.
    for entry in cache.kind_only_entries() {
        suggestions.push(nu_protocol::DynamicSuggestion {
            value: entry.kind.to_lowercase(),
            description: Some(format_group_version(&entry.group, &entry.version)),
            ..Default::default()
        });
    }

    for cat in cache.all_categories() {
        suggestions.push(nu_protocol::DynamicSuggestion {
            value: cat,
            description: Some("__category__".into()),
            ..Default::default()
        });
    }

    Ok(suggestions)
}

pub async fn complete_api_group(
    plugin: &NukePlugin,
    context: Option<String>,
    cluster: Option<String>,
    user: Option<String>,
) -> Result<Vec<nu_protocol::DynamicSuggestion>> {
    let config = kube::Config::from_kubeconfig(&kube::config::KubeConfigOptions {
        context,
        cluster,
        user,
    })
    .await?;
    let cache = plugin.discovery(&config)?;
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
    cluster: Option<String>,
    user: Option<String>,
) -> Result<Vec<nu_protocol::DynamicSuggestion>> {
    let config = kube::Config::from_kubeconfig(&kube::config::KubeConfigOptions {
        context,
        cluster,
        user,
    })
    .await?;

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
pub fn complete_clusters() -> Vec<nu_protocol::DynamicSuggestion> {
    let Ok(kubeconfig) = kube::config::Kubeconfig::read() else {
        return vec![];
    };

    kubeconfig
        .clusters
        .iter()
        .map(|c| nu_protocol::DynamicSuggestion {
            value: c.name.clone(),
            description: None,
            ..Default::default()
        })
        .collect()
}

pub fn complete_users() -> Vec<nu_protocol::DynamicSuggestion> {
    let Ok(kubeconfig) = kube::config::Kubeconfig::read() else {
        return vec![];
    };

    kubeconfig
        .auth_infos
        .iter()
        .map(|u| nu_protocol::DynamicSuggestion {
            value: u.name.clone(),
            description: None,
            ..Default::default()
        })
        .collect()
}

/// Complete instance names for a given resource type.
/// e.g. "pods" -> ["coredns-abc-123", "kube-proxy-xyz", ...]
pub async fn complete_resource_instances(
    plugin: &NukePlugin,
    resource: &str,
    namespace: Option<&str>,
    context: Option<String>,
    cluster: Option<String>,
    user: Option<String>,
) -> Result<Vec<nu_protocol::DynamicSuggestion>> {
    let config = kube::Config::from_kubeconfig(&kube::config::KubeConfigOptions {
        context,
        cluster,
        user,
    })
    .await?;

    let default_ns = config.default_namespace.clone();
    let client = Client::try_from(config.clone())?;

    let cache = plugin.discovery(&config)?;

    let entry = cache
        .find(resource)
        .ok_or_else(|| anyhow::anyhow!("unknown resource type: '{}'", resource))?;

    let ar = entry.to_api_resource();

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

// ---------------------------------------------------------------------------
// Label selector (`--labels`) completion
// ---------------------------------------------------------------------------

/// Aggregate the label keys and values observed across every instance of
/// `resource`, scoped to the selected context/cluster/user and `namespace`
/// (or every namespace when `all_namespaces`).
///
/// This is the candidate set for `--labels` completion — it reflects the
/// labels that objects actually carry, not a static list.
pub async fn complete_labels(
    plugin: &NukePlugin,
    resource: &str,
    namespace: Option<&str>,
    all_namespaces: bool,
    context: Option<String>,
    cluster: Option<String>,
    user: Option<String>,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let config = kube::Config::from_kubeconfig(&kube::config::KubeConfigOptions {
        context,
        cluster,
        user,
    })
    .await?;

    let default_ns = config.default_namespace.clone();
    let client = Client::try_from(config.clone())?;

    let cache = plugin.discovery(&config)?;
    let entry = cache
        .find(resource)
        .ok_or_else(|| anyhow::anyhow!("unknown resource type: '{}'", resource))?;

    let ar = entry.to_api_resource();
    let ns = namespace.unwrap_or(&default_ns);

    let api: Api<DynamicObject> = if all_namespaces || !entry.namespaced {
        Api::all_with(client, &ar)
    } else {
        Api::namespaced_with(client, ns, &ar)
    };

    let list = api.list(&ListParams::default()).await?;

    let mut labels: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for item in &list.items {
        for (k, v) in item.labels() {
            labels.entry(k.clone()).or_default().insert(v.clone());
        }
    }
    Ok(labels)
}

/// Return the text typed so far for the named flag, dropping the placeholder
/// grapheme nushell injects at the cursor when `call.strip` is set.
pub fn flag_prefix<'a>(call: &'a DynamicCompletionCall, name: &str) -> &'a str {
    let raw = flag_str(&call.call, name).unwrap_or_default();
    if call.strip {
        &raw[..raw.len().saturating_sub(1)]
    } else {
        raw
    }
}

/// Build `--labels` suggestions for the in-progress `typed` value.
///
/// The value is a comma-separated list of `key=value` pairs; only the segment
/// after the last comma is being edited. Everything before it is preserved
/// verbatim so multi-pair input round-trips.
///
///   - editing a key (no `=`)      → offer `…key=` for every known key
///   - editing a value (`key=va`)  → offer `…key=<each known value>`
///   - value already exact         → offer `…key=value,` to start the next pair
pub fn label_filter(
    typed: &str,
    labels: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<DynamicSuggestion> {
    let (committed, last) = match typed.rfind(',') {
        Some(i) => typed.split_at(i + 1),
        None => ("", typed),
    };

    match last.split_once('=') {
        Some((key, val)) => match labels.get(key) {
            Some(values) if !val.is_empty() && values.contains(val) => {
                vec![label_suggestion(format!("{typed},"))]
            }
            Some(values) => values
                .iter()
                .map(|v| label_suggestion(format!("{committed}{key}={v}")))
                .collect(),
            None => Vec::new(),
        },
        None => labels
            .keys()
            .map(|k| label_suggestion(format!("{committed}{k}=")))
            .collect(),
    }
}

/// A single `--labels` suggestion. `append_whitespace` is off so completing a
/// pair does not break the comma-separated syntax.
fn label_suggestion(value: String) -> DynamicSuggestion {
    DynamicSuggestion {
        value,
        append_whitespace: false,
        ..Default::default()
    }
}
