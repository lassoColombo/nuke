mod commands;
mod discovery;
mod plugin;

use nu_plugin::{MsgPackSerializer, serve_plugin};
use plugin::KubectlPlugin;

fn main() {
    serve_plugin(&KubectlPlugin::new(), MsgPackSerializer)
}
