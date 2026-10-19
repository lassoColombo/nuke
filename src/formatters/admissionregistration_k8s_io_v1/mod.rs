pub mod admissionregistration_helpers;
pub mod mutatingwebhookconfigurations;
pub mod validatingadmissionpolicies;
pub mod validatingadmissionpolicybindings;
pub mod validatingwebhookconfigurations;

use super::ResourceFormatter;

/// The API group these formatters serve.
pub const GROUP: &str = "admissionregistration.k8s.io";

/// `(version, plural, formatter)` for every resource of [`GROUP`] nuke formats.
pub const FORMATTERS: &[(&str, &str, &dyn ResourceFormatter)] = &[
    (
        "v1",
        "mutatingwebhookconfigurations",
        &mutatingwebhookconfigurations::MutatingWebhookConfigurationFormatter,
    ),
    (
        "v1",
        "validatingwebhookconfigurations",
        &validatingwebhookconfigurations::ValidatingWebhookConfigurationFormatter,
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
