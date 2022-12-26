use std::{iter::once, sync::Arc};

use fragments::{components::resources, events::EventState, Scope, Widget};
use winit::window::{Window, WindowBuilder};

use crate::{
    error::{Error, Result},
    events::{on_frame, on_resize},
    graphics_state, winit_request,
};

pub struct GraphicsState {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
}

impl GraphicsState {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(&*window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(Error::NoSuitableAdapter)?;

        // let adapter = instance
        //     .enumerate_adapters(wgpu::Backends::all())
        //     .filter(|adapter| {
        //         // Check if this adapter supports our surface
        //         !surface.get_supported_formats(&adapter).is_empty()
        //     })
        //     .next()
        //     .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None, // Trace path
            )
            .await?;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };

        surface.configure(&device, &config);

        // let modes = surface.get_supported_modes(&adapter);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
        })
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn draw(&mut self) -> Result<()> {
        let target = self.surface.get_current_texture()?;
        let view = target
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
        }

        self.queue.submit(once(encoder.finish()));
        target.present();

        Ok(())
    }
}

#[derive(Debug)]
pub struct GraphicsLayer {}

impl Widget for GraphicsLayer {
    fn render(self, scope: &mut Scope) {
        let window = scope
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

        let state = async {
            let window = Arc::new(window.await.unwrap());
            let state = GraphicsState::new(window).await?;
            tracing::info!("Intialized graphics state");

            Ok::<_, Error>(state)
        };

        scope.use_future(state, |scope, state| {
            if let Ok(state) = state {
                scope.set(graphics_state(), state);
            }
        });

        scope.on_event(on_resize(), |entity, &size| {
            if let Ok(mut state) = entity.get_mut(graphics_state()) {
                state.resize(size);
            }

            Default::default()
        });

        scope.on_event(on_frame(), |entity, &size| {
            if let Ok(mut state) = entity.get_mut(graphics_state()) {
                state.draw();
            }

            Default::default()
        });
    }
}
