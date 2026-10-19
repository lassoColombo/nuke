pub mod applications;
pub mod appprojects;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "argoproj.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    (
        "v1alpha1",
        "applications",
        &applications::ApplicationFormatter,
    ),
    ("v1alpha1", "appprojects", &appprojects::AppProjectFormatter),
];
