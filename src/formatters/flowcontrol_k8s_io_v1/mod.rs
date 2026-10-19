pub mod flowschemas;
pub mod prioritylevelconfigurations;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "flowcontrol.apiserver.k8s.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    ("v1", "flowschemas", &flowschemas::FlowSchemaFormatter),
    (
        "v1",
        "prioritylevelconfigurations",
        &prioritylevelconfigurations::PriorityLevelConfigurationFormatter,
    ),
];
