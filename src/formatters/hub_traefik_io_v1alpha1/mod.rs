//! Formatters for the `hub.traefik.io` API group (Traefik Hub — the API
//! management / gateway layer).
//!
//! All resources are `v1alpha1`. Most carry a common sync status
//! (`{ version, syncedAt }`, see `hub_helpers::sync_status`) and lack
//! `additionalPrinterColumns`, so compact columns are chosen for usefulness.
pub mod apiaccesses;
pub mod apibundles;
pub mod apicatalogitems;
pub mod apiplans;
pub mod apiportals;
pub mod apiratelimits;
pub mod apis;
pub mod apiversions;
pub mod hub_helpers;
pub mod managedapplications;
pub mod managedsubscriptions;

use super::ResourceFormatter;
use super::union_spec::UnionSpecFormatter;
use hub_helpers::hub_status;

/// The API group these formatters serve.
pub const GROUP: &str = "hub.traefik.io";

/// An `AccessControlPolicy` is one authentication method.
const ACCESS_CONTROL_POLICY: UnionSpecFormatter = UnionSpecFormatter {
    types: &[
        "apiKey",
        "basicAuth",
        "jwt",
        "oAuthIntro",
        "oidc",
        "oidcGoogle",
    ],
    flag: None,
    wide_extra: Some(hub_status),
};

/// An `AIService` is one AI provider; its CRD has no status.
const AI_SERVICE: UnionSpecFormatter = UnionSpecFormatter {
    types: &[
        "anthropic",
        "azureOpenai",
        "bedrock",
        "cohere",
        "deepSeek",
        "gemini",
        "mistral",
        "ollama",
        "openai",
        "qWen",
    ],
    flag: None,
    wide_extra: None,
};

/// An `APIAuth` is one auth method, `isDefault` marking the default one.
const API_AUTH: UnionSpecFormatter = UnionSpecFormatter {
    types: &["apiKey", "jwt", "ldap", "keyless"],
    flag: Some("isDefault"),
    wide_extra: Some(hub_status),
};

/// An `APIPortalAuth` is one portal auth method.
const API_PORTAL_AUTH: UnionSpecFormatter = UnionSpecFormatter {
    types: &["ldap", "oidc"],
    flag: None,
    wide_extra: Some(hub_status),
};

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    ("v1alpha1", "accesscontrolpolicies", &ACCESS_CONTROL_POLICY),
    ("v1alpha1", "aiservices", &AI_SERVICE),
    ("v1alpha1", "apiaccesses", &apiaccesses::APIAccessFormatter),
    ("v1alpha1", "apiauths", &API_AUTH),
    ("v1alpha1", "apibundles", &apibundles::APIBundleFormatter),
    (
        "v1alpha1",
        "apicatalogitems",
        &apicatalogitems::APICatalogItemFormatter,
    ),
    ("v1alpha1", "apiplans", &apiplans::APIPlanFormatter),
    ("v1alpha1", "apiportalauths", &API_PORTAL_AUTH),
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
