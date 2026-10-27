//! Formatter for `apps/v1 Deployment` resources.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};

use crate::formatters::ResourceFormatter;
use crate::formatters::helpers::{
    Container, LabelSelector, Strategy, append, fmt_containers, json_array, json_at, json_bool,
    json_i64, meta_annotation_int, spec_label_selector, spec_strategy,
};
use crate::rollout;

pub struct DeploymentFormatter;

// ---------------------------------------------------------------------------
// ResourceFormatter impl
// ---------------------------------------------------------------------------

#[derive(IntoValue)]
struct Compact {
    /// kubectl's rollout verdict, so `status` and `nuke rollout status`
    /// agree.
    status: &'static str,
    desired: i64,
    updated: i64,
    ready: i64,
    available: i64,
    unavailable: i64,
}

#[derive(IntoValue)]
struct Wide<'a> {
    selector: Option<LabelSelector<'a>>,
    strategy: Strategy<'a>,
    containers: Vec<Container<'a>>,
    revision: Option<i64>,
    paused: bool,
}

impl ResourceFormatter for DeploymentFormatter {
    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let count = |field| json_i64(data, &["status", field]).unwrap_or(0);
        let compact = Compact {
            status: rollout::status("Deployment", item),
            // spec.replicas defaults to 1.
            desired: json_i64(data, &["spec", "replicas"]).unwrap_or(1),
            updated: count("updatedReplicas"),
            ready: count("readyReplicas"),
            available: count("availableReplicas"),
            unavailable: count("unavailableReplicas"),
        };
        append(rec, compact, span);
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let data = &item.data;
        let wide = Wide {
            selector: spec_label_selector(data),
            strategy: spec_strategy(json_at(data, &["spec", "strategy"])),
            containers: fmt_containers(json_array(
                data,
                &["spec", "template", "spec", "containers"],
            )),
            revision: meta_annotation_int(item, "deployment.kubernetes.io/revision"),
            // `paused` defaults to false per the API.
            paused: json_bool(data, &["spec", "paused"]).unwrap_or(false),
        };
        append(rec, wide, span);
    }
}
