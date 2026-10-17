plugin_toolkit::instrument::bootstrap!();

use libation::LibationBackend;
use plugin_toolkit::plugin::Plugin;

fn main() -> plugin_toolkit::anyhow::Result<()> {
    Plugin::named("libation")
        .version(env!("CARGO_PKG_VERSION"))
        .service(LibationBackend::new("libation"))
        .serve()
}
