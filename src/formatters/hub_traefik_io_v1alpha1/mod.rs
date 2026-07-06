//! Formatters for the `hub.traefik.io` API group (Traefik Hub — the API
//! management / gateway layer).
//!
//! All resources are `v1alpha1`; every one is namespaced except
//! `AccessControlPolicy` (cluster-scoped).  Most carry a common sync status
//! (`{ version, syncedAt }`, see `hub_helpers::sync_status`) and lack
//! `additionalPrinterColumns`, so compact columns are chosen for usefulness.
pub mod hub_helpers;
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
pub mod managedapplications;
pub mod managedsubscriptions;
