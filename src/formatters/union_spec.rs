//! One formatter for every "union spec" CRD: a spec that holds exactly one
//! of a fixed set of variant keys (a Traefik `Middleware` is a `stripPrefix`
//! *or* a `headers` *or* …; a Hub `AccessControlPolicy` is a `jwt` *or* an
//! `oidc` *or* …). Compact names the configured variant as `type`; wide
//! carries the whole spec as `config`, so `$m.config.stripPrefix` stays
//! queryable without a column per variant.
//!
//! Each group's `mod.rs` registers one `const` instance per resource.

use kube::api::DynamicObject;
use nu_protocol::{IntoValue, Record, Span};
use serde_json::Value as Json;

use super::ResourceFormatter;
use super::helpers::{append, detected_type, json_at, json_bool, native_or_nothing};

pub struct UnionSpecFormatter {
    /// The variant keys, in the order the CRD defines them.
    pub types: &'static [&'static str],
    pub namespaced: bool,
    /// A spec bool shown in compact under its own name (an APIAuth's
    /// `isDefault`).
    pub flag: Option<&'static str>,
    /// Wide columns after `config` (Traefik Hub's `sync` and `conditions`).
    pub wide_extra: Option<fn(&DynamicObject, &mut Record, Span)>,
}

#[derive(IntoValue)]
struct Compact {
    r#type: Option<&'static str>,
}

#[derive(IntoValue)]
struct Wide {
    config: Option<Json>,
}

impl ResourceFormatter for UnionSpecFormatter {
    fn namespaced(&self) -> bool {
        self.namespaced
    }

    fn compact(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let compact = Compact {
            r#type: detected_type(&item.data, self.types),
        };
        append(rec, compact, span);
        // Named at runtime, so pushed rather than derived.
        if let Some(flag) = self.flag {
            rec.push(
                flag,
                json_bool(&item.data, &["spec", flag]).into_value(span),
            );
        }
    }

    fn wide(&self, item: &DynamicObject, rec: &mut Record, span: Span) {
        let wide = Wide {
            config: native_or_nothing(json_at(&item.data, &["spec"])),
        };
        append(rec, wide, span);
        if let Some(extra) = self.wide_extra {
            extra(item, rec, span);
        }
    }
}
