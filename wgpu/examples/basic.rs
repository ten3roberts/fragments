use fragments::{common::Container, App};
use fragments_wgpu::{GraphicsLayer, WinitBackend};
use tracing_subscriber::{prelude::*, EnvFilter};
use tracing_tree::HierarchicalLayer;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(HierarchicalLayer::new(4))
        .init();

    color_eyre::install()?;

    tracing::info!("Running");
    App::builder(WinitBackend {}).run(Container(GraphicsLayer {}))?;

    Ok(())
}
