//! Formatters for the `hub.traefik.io` API group (Traefik Hub — the API
//! management / gateway layer).
//!
//! All resources are `v1alpha1`; every one is namespaced except
//! `AccessControlPolicy` (cluster-scoped).  Most carry a common sync status
//! (`{ version, syncedAt }`, see `hub_helpers::sync_status`) and lack
//! `additionalPrinterColumns`, so compact columns are chosen for usefulness.
pub mod accesscontrolpolicies;
pub mod aiservices;
pub mod apiaccesses;
pub mod apiauths;
pub mod apibundles;
pub mod apicatalogitems;
pub mod apiplans;
pub mod apiportalauths;
pub mod apiportals;
pub mod apiratelimits;
pub mod apis;
pub mod apiversions;
pub mod hub_helpers;
pub mod managedapplications;
pub mod managedsubscriptions;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "hub.traefik.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    (
        "v1alpha1",
        "accesscontrolpolicies",
        &accesscontrolpolicies::AccessControlPolicyFormatter,
    ),
    ("v1alpha1", "aiservices", &aiservices::AIServiceFormatter),
    ("v1alpha1", "apiaccesses", &apiaccesses::APIAccessFormatter),
    ("v1alpha1", "apiauths", &apiauths::APIAuthFormatter),
    ("v1alpha1", "apibundles", &apibundles::APIBundleFormatter),
    (
        "v1alpha1",
        "apicatalogitems",
        &apicatalogitems::APICatalogItemFormatter,
    ),
    ("v1alpha1", "apiplans", &apiplans::APIPlanFormatter),
    (
        "v1alpha1",
        "apiportalauths",
        &apiportalauths::APIPortalAuthFormatter,
    ),
    ("v1alpha1", "apiportals", &apiportals::APIPortalFormatter),
    (
        "v1alpha1",
        "apiratelimits",
        &apiratelimits::APIRateLimitFormatter,
    ),
    ("v1alpha1", "apis", &apis::APIFormatter),
    ("v1alpha1", "apiversions", &apiversions::APIVersionFormatter),
    (
        "v1alpha1",
        "managedapplications",
        &managedapplications::ManagedApplicationFormatter,
    ),
    (
        "v1alpha1",
        "managedsubscriptions",
        &managedsubscriptions::ManagedSubscriptionFormatter,
    ),
];
