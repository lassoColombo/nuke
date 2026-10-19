pub mod deviceclasses;
pub mod resourceclaims;
pub mod resourceclaimtemplates;
pub mod resourceslices;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "resource.k8s.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    ("v1", "deviceclasses", &deviceclasses::DeviceClassFormatter),
    (
        "v1",
        "resourceclaims",
        &resourceclaims::ResourceClaimFormatter,
    ),
    (
        "v1",
        "resourceclaimtemplates",
        &resourceclaimtemplates::ResourceClaimTemplateFormatter,
    ),
    (
        "v1",
        "resourceslices",
        &resourceslices::ResourceSliceFormatter,
    ),
];
