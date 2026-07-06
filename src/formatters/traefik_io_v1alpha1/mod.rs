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
