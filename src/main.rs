plugin_toolkit::instrument::bootstrap!();

#[allow(unused_imports)]
use libation::tools as _;

fn main() -> plugin_toolkit::anyhow::Result<()> {
    plugin_toolkit::plugin::Plugin::named("libation")
        .version(env!("CARGO_PKG_VERSION"))
        .tools(["libation."])
        .service(libation::LibationBackend::new("libation"))
        .serve()
}
