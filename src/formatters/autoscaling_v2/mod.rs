pub mod autoscaling_helpers;
pub mod horizontalpodautoscalers;
pub mod horizontalpodautoscalers_v1;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "autoscaling";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    (
        "v1",
        "horizontalpodautoscalers",
        &horizontalpodautoscalers_v1::HorizontalPodAutoscalerV1Formatter,
    ),
    (
        "v2",
        "horizontalpodautoscalers",
        &horizontalpodautoscalers::HorizontalPodAutoscalerFormatter,
    ),
];
