pub mod alertmanagers;
pub mod monitoring_helpers;
pub mod podmonitors;
pub mod prometheuses;
pub mod prometheusrules;
pub mod servicemonitors;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "monitoring.coreos.com";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    ("v1", "alertmanagers", &alertmanagers::AlertmanagerFormatter),
    ("v1", "podmonitors", &podmonitors::PodMonitorFormatter),
    ("v1", "prometheuses", &prometheuses::PrometheusFormatter),
    (
        "v1alpha1",
        "prometheusagents",
        &prometheuses::PrometheusFormatter,
    ),
    (
        "v1",
        "prometheusrules",
        &prometheusrules::PrometheusRuleFormatter,
    ),
    (
        "v1",
        "servicemonitors",
        &servicemonitors::ServiceMonitorFormatter,
    ),
];
