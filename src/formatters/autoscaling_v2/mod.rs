pub mod horizontalpodautoscalers;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "autoscaling";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    (
        "v1",
        "horizontalpodautoscalers",
        &horizontalpodautoscalers::HorizontalPodAutoscalerFormatter,
    ),
    (
        "v2",
        "horizontalpodautoscalers",
        &horizontalpodautoscalers::HorizontalPodAutoscalerFormatter,
    ),
];
