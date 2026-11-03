mod cache;
mod columns;
mod commands;
mod completions;
mod decorators;
mod discovery;
mod kube_env;
mod plugin;
mod rollout;
mod values;

use nu_plugin::{MsgPackSerializer, serve_plugin};
use plugin::NukePlugin;

fn main() {
    serve_plugin(&NukePlugin::new(), MsgPackSerializer)
}
