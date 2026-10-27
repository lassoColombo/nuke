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
    /// Append the resource's compact columns (a `Compact` row). Default:
    /// none.
    fn compact(&self, _item: &DynamicObject, _rec: &mut Record, _span: Span) {}

    /// Append the resource's wide-only columns (a `Wide` row). Default:
    /// none.
    fn wide(&self, _item: &DynamicObject, _rec: &mut Record, _span: Span) {}

    /// The whole row, wide when `wide`. `namespaced` is discovery's word on
    /// the resource and decides whether the row carries a `namespace`.
    fn record(&self, item: &DynamicObject, namespaced: bool, span: Span, wide: bool) -> Record {
        let mut rec = Record::new();
        rec.push("name", meta_name(item).into_value(span));
        if namespaced {
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

pub(crate) fn key(group: &str, version: &str, plural: &str) -> String {
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
mod tests;
