pub mod csidrivers;
pub mod csinodes;
pub mod csistoragecapacities;
pub mod storageclasses;
pub mod volumeattachments;
pub mod volumeattributeclasses;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "storage.k8s.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    ("v1", "csidrivers", &csidrivers::CSIDriverFormatter),
    ("v1", "csinodes", &csinodes::CSINodeFormatter),
    (
        "v1",
        "csistoragecapacities",
        &csistoragecapacities::CSIStorageCapacityFormatter,
    ),
    (
        "v1",
        "storageclasses",
        &storageclasses::StorageClassFormatter,
    ),
    (
        "v1",
        "volumeattachments",
        &volumeattachments::VolumeAttachmentFormatter,
    ),
    (
        "v1",
        "volumeattributeclasses",
        &volumeattributeclasses::VolumeAttributesClassFormatter,
    ),
];
