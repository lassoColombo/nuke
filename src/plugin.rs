use crate::formatters::FormatterRegistry;
use tokio::runtime::Runtime;

pub struct NukePlugin {
    pub rt: Runtime,
    pub formatter_registry: FormatterRegistry,
}

impl NukePlugin {
    pub fn new() -> Self {
        Self {
            rt: Runtime::new().expect("failed to create tokio runtime"),
            formatter_registry: FormatterRegistry::new(),
        }
    }
}

impl nu_plugin::Plugin for NukePlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn commands(&self) -> Vec<Box<dyn nu_plugin::PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(crate::commands::get::GetCommand),
            Box::new(crate::commands::rollout_status::RolloutStatusCommand),
            Box::new(crate::commands::api_resources::ApiResourcesCommand),
            Box::new(crate::commands::api_versions::ApiVersionsCommand),
            Box::new(crate::commands::top::TopCommand),
            Box::new(crate::commands::http_get::HttpGetCommand),
            Box::new(crate::commands::config::config::ConfigCommand),
            Box::new(crate::commands::config::get_contexts::GetContextsCommand),
            Box::new(crate::commands::config::get_current_namespace::GetCurrentNamespaceCommand),
            Box::new(crate::commands::config::get_clusters::GetClustersCommand),
            Box::new(crate::commands::config::get_users::GetUsersCommand),
            Box::new(crate::commands::config::get_path::GetPathCommand),
            Box::new(crate::commands::config::switch_context::SwitchContextCommand),
            Box::new(crate::commands::config::switch_namespace::SwitchNamespaceCommand),
        ]
    }
}
