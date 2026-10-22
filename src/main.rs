mod commands;
mod completions;
mod conversions;
mod decorators;
mod discovery;
mod formatters;
mod kube_env;
mod plugin;
mod rollout;

use nu_plugin::{MsgPackSerializer, serve_plugin};
use plugin::NukePlugin;

fn main() {
    serve_plugin(&NukePlugin::new(), MsgPackSerializer)
}
