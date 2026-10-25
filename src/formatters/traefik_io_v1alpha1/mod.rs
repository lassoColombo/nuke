//! Formatters for the `traefik.io` API group (the Traefik proxy CRDs).
//!
//! All resources are namespaced, served under `v1alpha1`, and carry no status
//! or `additionalPrinterColumns` — so the compact columns here are chosen for
//! usefulness rather than mirroring kubectl (which would show only NAME/AGE).
pub mod ingressroutes;
pub mod serverstransports;
pub mod serverstransporttcps;
pub mod tlsoptions;
pub mod tlsstores;
pub mod traefik_helpers;

use super::ResourceFormatter;
use super::union_spec::UnionSpecFormatter;
use ingressroutes::IngressRouteFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "traefik.io";

/// A `Middleware` is one of these, in the order Traefik defines them.
const MIDDLEWARE: UnionSpecFormatter = UnionSpecFormatter {
    types: &[
        "addPrefix",
        "basicAuth",
        "buffering",
        "chain",
        "circuitBreaker",
        "compress",
        "contentType",
        "digestAuth",
        "encodedCharacters",
        "errors",
        "forwardAuth",
        "grpcWeb",
        "headers",
        "inFlightReq",
        "ipAllowList",
        "ipWhiteList",
        "passTLSClientCert",
        "plugin",
        "rateLimit",
        "redirectRegex",
        "redirectScheme",
        "replacePath",
        "replacePathRegex",
        "retry",
        "stripPrefix",
        "stripPrefixRegex",
        "tap",
    ],
    namespaced: true,
    flag: None,
    wide_extra: None,
};

/// A `MiddlewareTCP` is one of these.
const MIDDLEWARE_TCP: UnionSpecFormatter = UnionSpecFormatter {
    types: &["inFlightConn", "ipAllowList", "ipWhiteList"],
    namespaced: true,
    flag: None,
    wide_extra: None,
};

/// A `TraefikService` is one load balancer: `weighted` or `mirroring`, and
/// in newer Traefik releases `highestRandomWeight` or `failover`.
const TRAEFIK_SERVICE: UnionSpecFormatter = UnionSpecFormatter {
    types: &["weighted", "mirroring", "highestRandomWeight", "failover"],
    namespaced: true,
    flag: None,
    wide_extra: None,
};

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    (
        "v1alpha1",
        "ingressroutes",
        &IngressRouteFormatter {
            tls: true,
            parent_refs: true,
        },
    ),
    (
        "v1alpha1",
        "ingressroutetcps",
        &IngressRouteFormatter {
            tls: true,
            parent_refs: false,
        },
    ),
    (
        "v1alpha1",
        "ingressrouteudps",
        &IngressRouteFormatter {
            tls: false,
            parent_refs: false,
        },
    ),
    ("v1alpha1", "middlewares", &MIDDLEWARE),
    ("v1alpha1", "middlewaretcps", &MIDDLEWARE_TCP),
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
    ("v1alpha1", "traefikservices", &TRAEFIK_SERVICE),
];
