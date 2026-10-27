//! The formatter pipeline: `formatter_for(group, version, plural)` picks the
//! `ResourceFormatter` a group module registered for that exact GVR, else
//! [`default::DefaultFormatter`].

pub mod default;
pub mod helpers;
pub mod union_spec;

pub mod admissionregistration_k8s_io_v1;
pub mod apiextensions_k8s_io_v1;
pub mod apiregistration_k8s_io_v1;
pub mod apps_v1;
pub mod argoproj_io_v1alpha1;
pub mod autoscaling_v2;
pub mod batch_v1;
pub mod cert_manager_io_v1;
pub mod certificates_k8s_io_v1;
pub mod cilium_io_v2;
pub mod coordination_k8s_io_v1;
pub mod core_v1;
pub mod discovery_k8s_io_v1;
pub mod events_k8s_io_v1;
pub mod flowcontrol_k8s_io_v1;
pub mod gateway_networking_k8s_io_v1;
pub mod hub_traefik_io_v1alpha1;
pub mod monitoring_coreos_com_v1;
pub mod networking_k8s_io_v1;
pub mod node_k8s_io_v1;
pub mod policy_v1;
pub mod rbac_k8s_io_v1;
pub mod resource_k8s_io_v1;
pub mod scheduling_k8s_io_v1;
pub mod storage_k8s_io_v1;
pub mod traefik_io_v1alpha1;

/// Every group module's formatter table: each module exports `GROUP` and
/// `FORMATTERS`, and adding a group means adding its `pub mod` above and
/// its row here.
#[rustfmt::skip]
const GROUPS: &[(&str, FormatterTable)] = &[
    (admissionregistration_k8s_io_v1::GROUP, admissionregistration_k8s_io_v1::FORMATTERS),
    (apiextensions_k8s_io_v1::GROUP, apiextensions_k8s_io_v1::FORMATTERS),
    (apiregistration_k8s_io_v1::GROUP, apiregistration_k8s_io_v1::FORMATTERS),
    (apps_v1::GROUP, apps_v1::FORMATTERS),
    (argoproj_io_v1alpha1::GROUP, argoproj_io_v1alpha1::FORMATTERS),
    (autoscaling_v2::GROUP, autoscaling_v2::FORMATTERS),
    (batch_v1::GROUP, batch_v1::FORMATTERS),
    (cert_manager_io_v1::GROUP, cert_manager_io_v1::FORMATTERS),
    (certificates_k8s_io_v1::GROUP, certificates_k8s_io_v1::FORMATTERS),
    (cilium_io_v2::GROUP, cilium_io_v2::FORMATTERS),
    (coordination_k8s_io_v1::GROUP, coordination_k8s_io_v1::FORMATTERS),
    (core_v1::GROUP, core_v1::FORMATTERS),
    (discovery_k8s_io_v1::GROUP, discovery_k8s_io_v1::FORMATTERS),
    (events_k8s_io_v1::GROUP, events_k8s_io_v1::FORMATTERS),
    (flowcontrol_k8s_io_v1::GROUP, flowcontrol_k8s_io_v1::FORMATTERS),
    (gateway_networking_k8s_io_v1::GROUP, gateway_networking_k8s_io_v1::FORMATTERS),
    (hub_traefik_io_v1alpha1::GROUP, hub_traefik_io_v1alpha1::FORMATTERS),
    (monitoring_coreos_com_v1::GROUP, monitoring_coreos_com_v1::FORMATTERS),
    (networking_k8s_io_v1::GROUP, networking_k8s_io_v1::FORMATTERS),
    (node_k8s_io_v1::GROUP, node_k8s_io_v1::FORMATTERS),
    (policy_v1::GROUP, policy_v1::FORMATTERS),
    (rbac_k8s_io_v1::GROUP, rbac_k8s_io_v1::FORMATTERS),
    (resource_k8s_io_v1::GROUP, resource_k8s_io_v1::FORMATTERS),
    (scheduling_k8s_io_v1::GROUP, scheduling_k8s_io_v1::FORMATTERS),
    (storage_k8s_io_v1::GROUP, storage_k8s_io_v1::FORMATTERS),
    (traefik_io_v1alpha1::GROUP, traefik_io_v1alpha1::FORMATTERS),
];

use std::collections::HashMap;
use std::sync::LazyLock;

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use helpers::{meta_created, meta_name, meta_namespace, meta_owner};

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

impl std::str::FromStr for OutputFormat {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "compact" => Ok(Self::Compact),
            "wide" => Ok(Self::Wide),
            "full" => Ok(Self::Full),
            other => {
                anyhow::bail!("unknown output format '{other}': expected compact, wide or full")
            }
        }
    }
}

impl OutputFormat {
    /// The `--output` flag of `call`, rejecting unknown values rather than
    /// silently falling back to the default.
    pub fn from_flag(call: &nu_plugin::EvaluatedCall) -> anyhow::Result<Option<Self>> {
        call.get_flag::<String>("output")?
            .map(|s| s.parse())
            .transpose()
    }

    /// `compact` for lists, `wide` for a single named object.
    pub fn default_for(single: bool) -> Self {
        match single {
            true => Self::Wide,
            false => Self::Compact,
        }
    }
}

// ---------------------------------------------------------------------------
// Formatter trait
// ---------------------------------------------------------------------------

/// Turns a `DynamicObject` into a nushell record in two densities.
///
/// Every row starts with the identity columns and ends its compact part
/// with `created`; a formatter supplies only what lies between, and what
/// wide adds after `owner`:
///
/// ```text
/// compact: name, namespace (when namespaced), <compact columns>, created
/// wide:    the compact row, owner, <wide columns>
/// ```
///
/// Wide only appends, so a pipeline written against compact keeps working
/// under wide. Raw output (`-o full`) bypasses formatters entirely.
///
/// A formatter declares its columns as `#[derive(IntoValue)]` structs, by
/// convention `Compact` and `Wide` in its file, and hands one to
/// [`helpers::append`] from each method:
///
/// ```text
/// fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
///     append(rec, Compact { status: .., ready: .. }, span);
/// }
/// ```
pub trait ResourceFormatter: Send + Sync {
    /// Whether rows carry the object's `namespace`. Cluster-scoped
    /// resources say no.
    fn namespaced(&self) -> bool {
        true
    }

    /// Append the resource's compact columns (a `Compact` row). Default:
    /// none.
    fn compact(&self, _item: &DynamicObject, _rec: &mut Record, _span: Span) {}

    /// Append the resource's wide-only columns (a `Wide` row). Default:
    /// none.
    fn wide(&self, _item: &DynamicObject, _rec: &mut Record, _span: Span) {}

    /// The whole row, wide when `wide`.
    fn record(&self, item: &DynamicObject, span: Span, wide: bool) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item).into_value(span));
        if self.namespaced() {
            rec.push("namespace", meta_namespace(item).into_value(span));
        }
        self.compact(item, &mut rec, span);
        rec.push("created", meta_created(item).into_value(span));
        if wide {
            rec.push("owner", meta_owner(item).into_value(span));
            self.wide(item, &mut rec, span);
        }
        rec
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// A group module's `FORMATTERS`: `(version, plural, formatter)` rows.
pub type FormatterTable = &'static [(&'static str, &'static str, &'static dyn ResourceFormatter)];

type Registry = HashMap<String, &'static dyn ResourceFormatter>;

static REGISTRY: LazyLock<Registry> = LazyLock::new(|| {
    let mut map = Registry::new();
    for (group, table) in GROUPS {
        for (version, plural, formatter) in *table {
            let previous = map.insert(key(group, version, plural), *formatter);
            debug_assert!(
                previous.is_none(),
                "two formatters for {group}/{version}/{plural}"
            );
        }
    }
    map
});

fn key(group: &str, version: &str, plural: &str) -> String {
    format!("{group}/{version}/{plural}")
}

/// The formatter registered for an exact (group, version, plural) — an empty
/// group is the core API — or [`default::DefaultFormatter`].
pub fn formatter_for(group: &str, version: &str, plural: &str) -> &'static dyn ResourceFormatter {
    REGISTRY
        .get(&key(group, version, plural))
        .copied()
        .unwrap_or(&default::DefaultFormatter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::Value;
    use serde_json::json;

    /// Objects every formatter must handle: bare metadata, and metadata plus
    /// the usual sub-objects populated with the shapes formatters probe.
    fn samples() -> Vec<DynamicObject> {
        let bare = json!({"metadata": {"name": "x", "namespace": "ns"}});
        let rich = json!({
            "metadata": {
                "name": "x", "namespace": "ns", "generation": 2,
                "creationTimestamp": "2026-01-02T03:04:05Z",
                "labels": {"app": "a"}, "annotations": {"note": "n"},
                "ownerReferences": [{"apiVersion": "v1", "kind": "Owner", "name": "o", "uid": "u", "controller": true}]
            },
            "spec": {"replicas": 3, "selector": {"matchLabels": {"app": "a"}}, "ports": [{"port": 80, "targetPort": "http", "nodePort": 30080}],
                     "template": {"spec": {"containers": [{"name": "c", "image": "i", "resources": {"requests": {"cpu": "100m", "memory": "64Mi"}}}]}}},
            "status": {"replicas": 3, "readyReplicas": 2, "observedGeneration": 2,
                       "conditions": [{"type": "Ready", "status": "True", "reason": "R", "message": "m", "lastTransitionTime": "2026-01-02T03:04:05Z"}]},
            "data": {"k": "v"}, "count": 4, "value": 7, "revision": 5
        });
        [bare, rich]
            .into_iter()
            .map(|v| serde_json::from_value(v).unwrap())
            .collect()
    }

    /// The house contract: wide starts with every compact column, in order,
    /// with the same value, and never repeats a column. Pipelines written
    /// against compact keep working under wide.
    ///
    /// `Record::push` does not deduplicate, so a wide column named like a
    /// compact one would yield a record with two keys of that name.
    #[test]
    fn wide_starts_with_compact() {
        let span = Span::test_data();
        let mut violations = Vec::new();
        let default: &dyn ResourceFormatter = &default::DefaultFormatter;
        let formatters = REGISTRY
            .iter()
            .map(|(k, f)| (k.clone(), *f))
            .chain([("default".to_string(), default)]);
        for (key, f) in formatters {
            for (i, item) in samples().iter().enumerate() {
                let compact = f.record(item, span, false);
                let wide = f.record(item, span, true);
                let wide_prefix: Vec<_> = wide.iter().take(compact.len()).collect();
                let compact_cols: Vec<_> = compact.iter().collect();
                if wide_prefix != compact_cols {
                    let c: Vec<_> = compact.columns().collect();
                    let w: Vec<_> = wide.columns().take(compact.len()).collect();
                    violations.push(format!("{key} (sample {i}): compact {c:?} vs wide {w:?}"));
                }
                let mut seen = std::collections::HashSet::new();
                for col in wide.columns() {
                    if !seen.insert(col) {
                        violations.push(format!("{key} (sample {i}): duplicate column {col}"));
                    }
                }
            }
        }
        violations.sort();
        violations.dedup();
        assert!(
            violations.is_empty(),
            "{} violations:\n{}",
            violations.len(),
            violations.join("\n")
        );
    }

    /// A formatter column must not share a name with a `--show-*` decorator
    /// column unless it holds the same value: `decorators::apply` skips a
    /// column the formatter already produced, so a same-named column with
    /// other data (a namespace's `spec.finalizers`, a template's labels)
    /// would silently replace the decorator's.
    #[test]
    fn no_column_shadows_a_decorator() {
        let span = Span::test_data();
        let mut violations = Vec::new();
        for (key, f) in REGISTRY.iter() {
            for (i, item) in samples().iter().enumerate() {
                let wide = f.record(item, span, true);
                for d in crate::decorators::DECORATORS {
                    if let Some(v) = wide.get(d.column)
                        && *v != (d.value)(item, span)
                    {
                        violations.push(format!("{key} (sample {i}): column {}", d.column));
                    }
                }
            }
        }
        violations.sort();
        assert!(
            violations.is_empty(),
            "{} formatter columns shadow a decorator with different data:\n{}",
            violations.len(),
            violations.join("\n")
        );
    }

    /// Dev check: every registered `(group, version, plural)` is served by at
    /// least one cluster kubectl has a discovery cache for on this machine.
    /// A plural or version typo makes a formatter unreachable, and nothing
    /// in the synthetic tests can tell; the cache (the same one `nuke get`
    /// resolves names from) can.
    ///
    /// ```text
    /// cargo test registry_matches_discovery_cache -- --ignored
    /// ```
    ///
    /// Layout, as kubectl writes it: `<cache>/discovery/<host>/<group>/
    /// <version>/serverresources.json`, with the core group's directory
    /// being just `<version>`.
    #[test]
    #[ignore]
    fn registry_matches_discovery_cache() {
        let root = std::env::var_os("KUBECACHEDIR")
            .filter(|v| !v.is_empty())
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".kube/cache"))
            })
            .expect("neither KUBECACHEDIR nor HOME is set")
            .join("discovery");
        let hosts =
            std::fs::read_dir(&root).expect("no discovery cache; run kubectl api-resources first");
        let mut served = std::collections::HashSet::new();
        for host in hosts.flatten() {
            for (group, version) in group_versions(&host.path()) {
                let file = host
                    .path()
                    .join(group_dir(&group, &version))
                    .join("serverresources.json");
                let Ok(bytes) = std::fs::read(file) else {
                    continue;
                };
                let Ok(doc) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
                    continue;
                };
                for r in doc["resources"].as_array().into_iter().flatten() {
                    if let Some(name) = r["name"].as_str() {
                        served.insert(key(&group, &version, name));
                    }
                }
            }
        }
        assert!(
            !served.is_empty(),
            "no serverresources.json under {}",
            root.display()
        );
        let mut unserved: Vec<_> = REGISTRY
            .keys()
            .filter(|k| !served.contains(*k))
            .cloned()
            .collect();
        unserved.sort();
        assert!(
            unserved.is_empty(),
            "{} registered resources are served by no cached cluster (a plural/version typo, or a CRD no local cluster installs):\n{}",
            unserved.len(),
            unserved.join("\n")
        );
    }

    /// Every `(group, version)` a host's cache directory holds, the core
    /// group's `<version>` directories included.
    fn group_versions(host: &std::path::Path) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(host).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !entry.path().is_dir() {
                continue;
            }
            if entry.path().join("serverresources.json").is_file() {
                out.push((String::new(), name));
                continue;
            }
            for v in std::fs::read_dir(entry.path())
                .into_iter()
                .flatten()
                .flatten()
            {
                out.push((name.clone(), v.file_name().to_string_lossy().into_owned()));
            }
        }
        out
    }

    fn group_dir(group: &str, version: &str) -> std::path::PathBuf {
        match group {
            "" => std::path::PathBuf::from(version),
            g => std::path::PathBuf::from(g).join(version),
        }
    }

    /// Dev tool for formatter refactors: run every formatter over a saved
    /// dump of raw objects and write their compact and wide output, so two
    /// builds can be diffed on identical input.
    ///
    /// Input (`NUKE_GOLDEN_IN`): a JSON list of
    /// `{group, version, plural, items: [raw object, ...]}`, e.g. from
    /// `nuke get <group/version/plural> -o full`. Output (`NUKE_GOLDEN_OUT`):
    /// plain text, each value in exact `Debug` form, ready for `diff`.
    ///
    /// ```text
    /// NUKE_GOLDEN_IN=in.json NUKE_GOLDEN_OUT=before.txt cargo test golden_dump -- --ignored
    /// ```
    #[test]
    #[ignore]
    fn golden_dump() {
        use std::fmt::Write;
        let input = std::env::var("NUKE_GOLDEN_IN").expect("set NUKE_GOLDEN_IN");
        let output = std::env::var("NUKE_GOLDEN_OUT").expect("set NUKE_GOLDEN_OUT");
        let groups: Vec<serde_json::Value> =
            serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
        let span = Span::test_data();
        let mut out = String::new();
        for g in &groups {
            let field = |k: &str| g[k].as_str().unwrap_or_default();
            let f = formatter_for(field("group"), field("version"), field("plural"));
            for raw in g["items"].as_array().into_iter().flatten() {
                let item: DynamicObject = serde_json::from_value(raw.clone()).unwrap();
                for mode in [OutputFormat::Compact, OutputFormat::Wide] {
                    let header = format!(
                        "{}/{}/{} {:?}",
                        field("group"),
                        field("version"),
                        field("plural"),
                        mode
                    );
                    let _ = writeln!(
                        out,
                        "=== {header} {}\n{:#?}",
                        item.metadata.name.as_deref().unwrap_or(""),
                        Value::record(f.record(&item, span, mode == OutputFormat::Wide), span)
                    );
                }
            }
        }
        // Every registered formatter over the synthetic samples too, so
        // resources absent from the dump are still covered.
        let mut keys: Vec<_> = REGISTRY.keys().collect();
        keys.sort_by_key(|k| k.split('/').map(str::to_owned).collect::<Vec<_>>());
        for k in keys {
            let f = REGISTRY[k];
            for (i, item) in samples().iter().enumerate() {
                for mode in [OutputFormat::Compact, OutputFormat::Wide] {
                    let _ = writeln!(
                        out,
                        "=== synthetic {k} {:?} {i}\n{:#?}",
                        mode,
                        Value::record(f.record(item, span, mode == OutputFormat::Wide), span)
                    );
                }
            }
        }
        std::fs::write(output, out).unwrap();
    }
}
