pub mod runtimeclasses;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "node.k8s.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[(
    "v1",
    "runtimeclasses",
    &runtimeclasses::RuntimeClassFormatter,
)];
