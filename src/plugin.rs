use crate::formatters::FormatterRegistry;
use tokio::runtime::Runtime;

pub struct KubectlPlugin {
    pub rt: Runtime,
    pub formatter_registry: FormatterRegistry,
}

impl KubectlPlugin {
    pub fn new() -> Self {
        Self {
            rt: Runtime::new().expect("failed to create tokio runtime"),
            formatter_registry: FormatterRegistry::new(),
        }
    }
}

impl nu_plugin::Plugin for KubectlPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn commands(&self) -> Vec<Box<dyn nu_plugin::PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(crate::commands::get::GetCommand),
            Box::new(crate::commands::api_resources::ApiResourcesCommand),
            Box::new(crate::commands::api_versions::ApiVersionsCommand),
            Box::new(crate::commands::top::TopCommand),
        ]
    }
}
