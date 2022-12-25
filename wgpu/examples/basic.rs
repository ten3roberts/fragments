use std::{sync::Arc, time::Duration};

use fragments::{components::resources, App, Scope};
use fragments_wgpu::{graphics_state, winit_request, GraphicsState, WinitBackend};
use tokio::time::sleep;
use tracing_subscriber::{prelude::*, EnvFilter};
use tracing_tree::HierarchicalLayer;
use winit::window::WindowBuilder;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(HierarchicalLayer::new(4))
        .init();

    color_eyre::install()?;

    tracing::info!("Running");
    App::builder(WinitBackend {}).run(|s: &mut Scope| {
        let window = s
            .entity_mut()
            .world()
            .get_mut(resources(), winit_request())
            .unwrap()
            .request_window(|| {
                WindowBuilder::new()
                    .with_visible(true)
                    .with_decorations(true)
                    .with_title("fragments")
            });

        tracing::info!("Requested window");
        s.use_future(
            async move {
                let window = Arc::new(window.await.unwrap());

                sleep(Duration::from_millis(5000)).await;
                tracing::info!("Got window");

                tracing::info!("Opened window");
                GraphicsState::new(window).await.unwrap()
            },
            |s, state| {
                s.entity_mut()
                    .world_mut()
                    .set(resources(), graphics_state(), Arc::new(state))
                    .unwrap();
            },
        );
    })?;

    Ok(())
}
