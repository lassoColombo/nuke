//! Formatters for the `traefik.io` API group (the Traefik proxy CRDs).
//!
//! All resources are namespaced, served under `v1alpha1`, and carry no status
//! or `additionalPrinterColumns` — so the compact columns here are chosen for
//! usefulness rather than mirroring kubectl (which would show only NAME/AGE).
pub mod ingressroutes;
pub mod ingressroutetcps;
pub mod ingressrouteudps;
pub mod middlewares;
pub mod middlewaretcps;
pub mod serverstransports;
pub mod serverstransporttcps;
pub mod tlsoptions;
pub mod tlsstores;
pub mod traefikservices;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "traefik.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    (
        "v1alpha1",
        "ingressroutes",
        &ingressroutes::IngressRouteFormatter,
    ),
    (
        "v1alpha1",
        "ingressroutetcps",
        &ingressroutetcps::IngressRouteTCPFormatter,
    ),
    (
        "v1alpha1",
        "ingressrouteudps",
        &ingressrouteudps::IngressRouteUDPFormatter,
    ),
    ("v1alpha1", "middlewares", &middlewares::MiddlewareFormatter),
    (
        "v1alpha1",
        "middlewaretcps",
        &middlewaretcps::MiddlewareTCPFormatter,
    ),
    (
        "v1alpha1",
        "serverstransports",
        &serverstransports::ServersTransportFormatter,
    ),
    (
        "v1alpha1",
        "serverstransporttcps",
        &serverstransporttcps::ServersTransportTCPFormatter,
    ),
    ("v1alpha1", "tlsoptions", &tlsoptions::TLSOptionFormatter),
    ("v1alpha1", "tlsstores", &tlsstores::TLSStoreFormatter),
    (
        "v1alpha1",
        "traefikservices",
        &traefikservices::TraefikServiceFormatter,
    ),
];
