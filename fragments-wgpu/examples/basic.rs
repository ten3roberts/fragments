use fragments_wgpu::app::{App, AppBuilder};
use tracing_subscriber::prelude::*;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(HierarchialLayer::new(4))
        .init();
    AppBuilder::new().build().run()
}
