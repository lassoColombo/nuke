mod commands;
mod completions;
mod discovery;
mod formatters;
mod plugin;
mod types;

use nu_plugin::{serve_plugin, MsgPackSerializer};
use plugin::NukePlugin;

fn main() {
    serve_plugin(&NukePlugin::new(), MsgPackSerializer)
}
