use fragments_wgpu::app::{App, AppBuilder};
use tracing_subscriber::prelude::*;
use tracing_tree::HierarchicalLayer;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(HierarchicalLayer::new(4))
        .init();
    AppBuilder::new().build().run()
}
