//! Dynamic completions shared by the commands.
//!
//! Completions fire per keystroke, so everything here is best-effort: errors
//! become "no suggestions", and discovery is a pure read of kubectl's cache
//! ([`NukePlugin::discovery`]) that never spawns kubectl.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use k8s_openapi::api::core::v1::Namespace;
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::{
    Api, ResourceExt,
    api::{DynamicObject, ListParams},
};
use nu_plugin::DynamicCompletionCall;
use nu_protocol::DynamicSuggestion;
use nu_protocol::ast::{Call, Expr, Expression};

use crate::kube_env::KubeEnv;
use crate::plugin::NukePlugin;

// ---------------------------------------------------------------------------
// Flags every cluster command shares
// ---------------------------------------------------------------------------

/// Complete `--context`, `--cluster`, `--user`, `--namespace` and `--output`,
/// the flags most commands share; `None` for any other flag so the command can
/// handle its own.
pub fn complete_common_flag(
    plugin: &NukePlugin,
    env: &KubeEnv,
    call: &DynamicCompletionCall,
    flag: &str,
) -> Option<Vec<DynamicSuggestion>> {
    match flag {
        "context" => Some(kubeconfig_names(env, |kc| {
            kc.contexts.iter().map(|c| c.name.clone()).collect()
        })),
        "cluster" => Some(kubeconfig_names(env, |kc| {
            kc.clusters.iter().map(|c| c.name.clone()).collect()
        })),
        "user" => Some(kubeconfig_names(env, |kc| {
            kc.auth_infos.iter().map(|u| u.name.clone()).collect()
        })),
        "output" => Some(complete_output()),
        "namespace" => Some(
            plugin
                .rt
                .block_on(complete_namespaces(env, &selection(&call.call)))
                .unwrap_or_default(),
        ),
        _ => None,
    }
}

/// The `--context` / `--cluster` / `--user` typed so far, as a selection.
pub fn selection(call: &Call) -> KubeConfigOptions {
    let flag = |name| flag_str(call, name).map(str::to_string);
    KubeConfigOptions {
        context: flag("context"),
        cluster: flag("cluster"),
        user: flag("user"),
    }
}

fn kubeconfig_names(
    env: &KubeEnv,
    names: fn(&Kubeconfig) -> Vec<String>,
) -> Vec<DynamicSuggestion> {
    let Ok(kubeconfig) = env.read_kubeconfig() else {
        return vec![];
    };
    names(&kubeconfig).into_iter().map(suggestion).collect()
}

async fn complete_namespaces(
    env: &KubeEnv,
    sel: &KubeConfigOptions,
) -> Result<Vec<DynamicSuggestion>> {
    let (_, client) = env.connect(sel).await?;
    let list = Api::<Namespace>::all(client)
        .list(&ListParams::default())
        .await?;
    Ok(list
        .items
        .iter()
        .map(|ns| suggestion(ns.name_any()))
        .collect())
}

fn complete_output() -> Vec<DynamicSuggestion> {
    [
        ("compact", "Flat row of primitive columns"),
        ("wide", "Compact plus extended and nested columns"),
        ("full", "The raw object from the apiserver"),
    ]
    .into_iter()
    .map(|(value, description)| DynamicSuggestion {
        value: value.into(),
        description: Some(description.into()),
        ..Default::default()
    })
    .collect()
}

// ---------------------------------------------------------------------------
// Resource types and instances
// ---------------------------------------------------------------------------

/// Every name the cluster serves: primary resources by plural, kind-only ones
/// (e.g. PodMetrics) by kind, and categories.
pub async fn complete_resource_names(
    plugin: &NukePlugin,
    env: &KubeEnv,
    sel: &KubeConfigOptions,
) -> Result<Vec<DynamicSuggestion>> {
    let config = env.config(sel).await?;
    let cache = plugin.discovery(env, &config)?;
    let described = |value: String, description: String| DynamicSuggestion {
        value,
        description: Some(description),
        ..Default::default()
    };

    let primary = cache
        .entries()
        .map(|e| described(e.plural.clone(), group_version(e)));
    let kind_only = cache
        .kind_only_entries()
        .map(|e| described(e.kind.to_lowercase(), group_version(e)));
    let categories = cache
        .all_categories()
        .into_iter()
        .map(|c| described(c, "category".into()));
    Ok(primary.chain(kind_only).chain(categories).collect())
}

/// `core/v1`, `apps/v1`: the description shown next to a resource name.
fn group_version(e: &crate::discovery::ResourceEntry) -> String {
    let group = if e.group.is_empty() { "core" } else { &e.group };
    format!("{group}/{}", e.version)
}

/// Every non-core API group the cluster serves (for `--group`).
pub async fn complete_api_group(
    plugin: &NukePlugin,
    env: &KubeEnv,
    sel: &KubeConfigOptions,
) -> Result<Vec<DynamicSuggestion>> {
    let config = env.config(sel).await?;
    let cache = plugin.discovery(env, &config)?;
    let groups: BTreeSet<&str> = cache
        .served()
        .map(|e| e.group.as_str())
        .filter(|g| !g.is_empty())
        .collect();
    Ok(groups.into_iter().map(suggestion).collect())
}

/// Instance names of `resource` in `namespace` (default: the context's).
pub async fn complete_resource_instances(
    plugin: &NukePlugin,
    env: &KubeEnv,
    resource: &str,
    namespace: Option<&str>,
    sel: &KubeConfigOptions,
) -> Result<Vec<DynamicSuggestion>> {
    let items = list_objects(plugin, env, resource, namespace, false, sel).await?;
    Ok(items
        .iter()
        .map(|item| suggestion(item.name_any()))
        .collect())
}

/// The label keys and values actually carried by instances of `resource` in
/// `namespace` (default: the context's), or in every namespace when
/// `all_namespaces` — the candidate set for `--labels`.
pub async fn complete_labels(
    plugin: &NukePlugin,
    env: &KubeEnv,
    resource: &str,
    namespace: Option<&str>,
    all_namespaces: bool,
    sel: &KubeConfigOptions,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let items = list_objects(plugin, env, resource, namespace, all_namespaces, sel).await?;
    let mut labels: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for item in &items {
        for (k, v) in item.labels() {
            labels.entry(k.clone()).or_default().insert(v.clone());
        }
    }
    Ok(labels)
}

async fn list_objects(
    plugin: &NukePlugin,
    env: &KubeEnv,
    resource: &str,
    namespace: Option<&str>,
    all_namespaces: bool,
    sel: &KubeConfigOptions,
) -> Result<Vec<DynamicObject>> {
    let (config, client) = env.connect(sel).await?;
    let cache = plugin.discovery(env, &config)?;
    let entry = cache
        .find(resource)
        .ok_or_else(|| anyhow::anyhow!("unknown resource type: '{resource}'"))?;
    let namespace = match all_namespaces {
        true => None,
        false => Some(namespace.unwrap_or(&config.default_namespace)),
    };
    Ok(entry
        .api(client, namespace)
        .list(&ListParams::default())
        .await?
        .items)
}

fn suggestion(value: impl Into<String>) -> DynamicSuggestion {
    DynamicSuggestion {
        value: value.into(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// AST helpers for dynamic completions
// ---------------------------------------------------------------------------

pub fn expr_as_str(expr: &Expression) -> Option<&str> {
    match &expr.expr {
        Expr::String(s) => Some(s.as_str()),
        Expr::GlobPattern(s, _) => Some(s.as_str()),
        _ => None,
    }
}

/// The string value of flag `name` as typed so far.
pub fn flag_str<'a>(call: &'a Call, name: &str) -> Option<&'a str> {
    call.named_iter()
        .find(|(n, _, _)| n.item == name)
        .and_then(|(_, _, expr)| expr.as_ref())
        .and_then(expr_as_str)
}

/// The `n`th positional argument as typed so far.
pub fn positional_str(call: &Call, n: usize) -> Option<&str> {
    call.positional_iter().nth(n).and_then(expr_as_str)
}

// ---------------------------------------------------------------------------
// Label selector (`--labels`) completion
// ---------------------------------------------------------------------------

/// Return the text typed so far for the named flag, dropping the placeholder
/// grapheme nushell injects at the cursor when `call.strip` is set.
pub fn flag_prefix<'a>(call: &'a DynamicCompletionCall, name: &str) -> &'a str {
    let raw = flag_str(&call.call, name).unwrap_or_default();
    match call.strip {
        true => raw
            .char_indices()
            .next_back()
            .map_or(raw, |(i, _)| &raw[..i]),
        false => raw,
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
