pub mod apiservices;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "apiregistration.k8s.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] =
    &[("v1", "apiservices", &apiservices::APIServiceFormatter)];
