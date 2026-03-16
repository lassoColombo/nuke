use nu_plugin::{Plugin, PluginCommand};
use tokio::runtime::Runtime;

use crate::commands::get::GetCommand;

pub struct KubectlPlugin {
    pub rt: Runtime,
}

impl KubectlPlugin {
    pub fn new() -> KubectlPlugin {
        KubectlPlugin {
            rt: Runtime::new().expect("failed to create tokio runtime"),
        }
    }
}

impl Plugin for KubectlPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(GetCommand),
        ]
    }
}
