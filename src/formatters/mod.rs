pub mod core_v1;
pub mod default;

use kube::api::DynamicObject;
use nu_protocol::{Span, Value};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Output format
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Compact,
    Wide,
    Full,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "compact" => Some(Self::Compact),
            "wide" => Some(Self::Wide),
            "full" => Some(Self::Full),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Formatter trait
// ---------------------------------------------------------------------------

/// A formatter knows how to turn a `DynamicObject` into a nushell `Value`
/// in two display densities.
pub trait ResourceFormatter: Send + Sync {
    fn format_compact(&self, item: &DynamicObject, span: Span) -> Value;

    fn format_wide(&self, item: &DynamicObject, span: Span) -> Value {
        self.format_compact(item, span)
    }

    /// Format according to the requested `OutputFormat`.
    /// `Full` is handled *before* this is called (in `run_get`), so this
    /// method only needs to handle `Compact` and `Wide`.
    fn format(&self, item: &DynamicObject, span: Span, mode: OutputFormat) -> Value {
        match mode {
            OutputFormat::Wide => self.format_wide(item, span),
            OutputFormat::Compact => self.format_compact(item, span),
            OutputFormat::Full => unreachable!("Full must be handled before formatter dispatch"),
        }
    }
}

// ---------------------------------------------------------------------------
// Registry key
// ---------------------------------------------------------------------------

/// Canonical lookup key: (group, version, plural).
/// An empty `group` string means the core API group.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FormatterKey {
    pub group: String,
    pub version: String,
    pub plural: String,
}

impl FormatterKey {
    pub fn new(
        group: impl Into<String>,
        version: impl Into<String>,
        plural: impl Into<String>,
    ) -> Self {
        Self {
            group: group.into(),
            version: version.into(),
            plural: plural.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Holds all registered formatters indexed by `FormatterKey`.
pub struct FormatterRegistry {
    formatters: HashMap<FormatterKey, Box<dyn ResourceFormatter>>,
    default: Box<dyn ResourceFormatter>,
}

impl FormatterRegistry {
    /// Build the global registry with all built-in formatters registered.
    pub fn new() -> Self {
        let mut reg = Self {
            formatters: HashMap::new(),
            default: Box::new(default::DefaultFormatter),
        };
        reg.register_builtins();
        reg
    }

    /// Register a formatter for an exact GVR triple.
    pub fn register(&mut self, key: FormatterKey, formatter: impl ResourceFormatter + 'static) {
        self.formatters.insert(key, Box::new(formatter));
    }

    /// Look up a formatter.  Resolution order:
    ///   1. Exact (group, version, plural)
    ///   2. Wildcard version  (group, "*", plural)
    ///   3. Built-in default
    pub fn get(&self, group: &str, version: &str, plural: &str) -> &dyn ResourceFormatter {
        let exact = FormatterKey::new(group, version, plural);
        if let Some(f) = self.formatters.get(&exact) {
            return f.as_ref();
        }

        let wildcard = FormatterKey::new(group, "*", plural);
        if let Some(f) = self.formatters.get(&wildcard) {
            return f.as_ref();
        }

        self.default.as_ref()
    }

    // -----------------------------------------------------------------------
    // Built-in registrations
    // -----------------------------------------------------------------------

    fn register_builtins(&mut self) {
        use core_v1::pods::PodFormatter;
        self.register(FormatterKey::new("", "v1", "pods"), PodFormatter);
    }
}

impl Default for FormatterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
