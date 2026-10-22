pub mod admissionregistration_helpers;
pub mod validatingadmissionpolicies;
pub mod validatingadmissionpolicybindings;
pub mod webhookconfigurations;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "admissionregistration.k8s.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    (
        "v1",
        "mutatingwebhookconfigurations",
        &webhookconfigurations::WebhookConfigurationFormatter,
    ),
    (
        "v1",
        "validatingwebhookconfigurations",
        &webhookconfigurations::WebhookConfigurationFormatter,
    ),
    (
        "v1",
        "validatingadmissionpolicies",
        &validatingadmissionpolicies::ValidatingAdmissionPolicyFormatter,
    ),
    (
        "v1",
        "validatingadmissionpolicybindings",
        &validatingadmissionpolicybindings::ValidatingAdmissionPolicyBindingFormatter,
    ),
];
