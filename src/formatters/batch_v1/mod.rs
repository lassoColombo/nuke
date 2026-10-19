pub mod cronjobs;
pub mod jobs;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "batch";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    ("v1", "jobs", &jobs::JobFormatter),
    ("v1", "cronjobs", &cronjobs::CronJobFormatter),
];
