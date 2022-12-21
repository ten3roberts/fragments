use std::sync::Arc;

use fragments::{components::resources, signal::from_future, App, Scope};
use fragments_wgpu::{graphics_state, winit_request, GraphicsState, WinitBackend};
use tracing_subscriber::prelude::*;
use tracing_tree::HierarchicalLayer;
use winit::window::WindowBuilder;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    tracing_subscriber::registry()
        .with(HierarchicalLayer::new(4))
        .init();

    color_eyre::install()?;

    tracing::info!("Running");
    App::builder(WinitBackend {
        root: |mut s: Scope| {
            let window = s
                .entity_mut()
                .world()
                .get_mut(resources(), winit_request())
                .unwrap()
                .request_window(|| WindowBuilder::new().with_title("fragments"));

            s.use_future(
                async move {
                    let window = Arc::new(window.await.unwrap());

                    tracing::info!("Opened window");
                    GraphicsState::new(window).await.unwrap()
                },
                |s, state| {
                    s.entity_mut()
                        .world_mut()
                        .set(resources(), graphics_state(), Arc::new(state))
                        .unwrap();
                },
            )
        },
    })
    .run()?;

    Ok(())
}
