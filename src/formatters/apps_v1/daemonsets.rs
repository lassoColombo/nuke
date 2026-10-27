//! Formatter for `apps/v1 DaemonSet` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Container, LabelSelector, StrMap, Strategy, append, fmt_containers, json_array, json_at,
    json_i64, json_str_map, meta_annotation_int, spec_label_selector, spec_strategy,
};
use crate::rollout;

pub struct DaemonSetFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Compact<'a> {
    /// kubectl's rollout verdict, so `status` and `nuke rollout status`
    /// agree.
    status: &'static str,
    desired: i64,
    current: i64,
    ready: i64,
    updated: i64,
    available: i64,
    unavailable: i64,
    /// kubectl's NODE SELECTOR: the template's, as a record.
    node_selector: StrMap<'a>,
}

#[derive(IntoValue)]
#[nu_value(rename_all = "camelCase")]
struct Wide<'a> {
    selector: Option<LabelSelector<'a>>,
    strategy: Strategy<'a>,
    containers: Vec<Container<'a>>,
    template_generation: Option<i64>,
}

impl ResourceFormatter for DaemonSetFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let count = |field| json_i64(data, &["status", field]).unwrap_or(0);
        let compact = Compact {
            status: rollout::status("DaemonSet", item),
            desired: count("desiredNumberScheduled"),
            current: count("currentNumberScheduled"),
            ready: count("numberReady"),
            updated: count("updatedNumberScheduled"),
            available: count("numberAvailable"),
            unavailable: count("numberUnavailable"),
            node_selector: json_str_map(data, &["spec", "template", "spec", "nodeSelector"]),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            selector: spec_label_selector(data),
            // DaemonSets keep theirs under spec.updateStrategy, not spec.strategy.
            strategy: spec_strategy(json_at(data, &["spec", "updateStrategy"])),
            containers: fmt_containers(json_array(
                data,
                &["spec", "template", "spec", "containers"],
            )),
            template_generation: meta_annotation_int(
                item,
                "deprecated.daemonset.template.generation",
            ),
        };
        append(rec, wide, span);
    }
}
