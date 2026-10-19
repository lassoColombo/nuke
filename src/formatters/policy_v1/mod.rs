pub mod poddisruptionbudgets;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "policy";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[(
    "v1",
    "poddisruptionbudgets",
    &poddisruptionbudgets::PodDisruptionBudgetFormatter,
)];
