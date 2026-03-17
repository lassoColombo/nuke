mod client;
mod commands;
mod completions;
mod discovery;
mod formatters;
mod plugin;

use nu_plugin::{serve_plugin, MsgPackSerializer};
use plugin::KubectlPlugin;

fn main() {
    serve_plugin(&KubectlPlugin::new(), MsgPackSerializer)
}
