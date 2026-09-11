mod commands;
mod completions;
mod conversions;
mod decorators;
mod discovery;
mod formatters;
mod kube_env;
mod plugin;

use nu_plugin::{serve_plugin, MsgPackSerializer};
use plugin::NukePlugin;

fn main() {
    serve_plugin(&NukePlugin::new(), MsgPackSerializer)
}
