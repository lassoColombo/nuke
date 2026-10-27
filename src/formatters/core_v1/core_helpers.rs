//! Shared helpers for the `core/v1` formatters.

use kube::api::DynamicObject;

use crate::formatters::helpers::{json_str, meta_annotation};

/// The storage class of a PersistentVolume or PersistentVolumeClaim, as
/// kubectl's `GetPersistentVolume(Claim)Class` reads it: the legacy
/// `volume.beta.kubernetes.io/storage-class` annotation first, then
/// `spec.storageClassName`. A claim's empty string is kept: it means "no
/// dynamic provisioning, bind a pre-provisioned volume", which an absent
/// class (use the default StorageClass) does not.
pub fn storage_class(item: &DynamicObject) -> Option<&str> {
    meta_annotation(item, "volume.beta.kubernetes.io/storage-class")
        .or_else(|| json_str(&item.data, &["spec", "storageClassName"]))
}
