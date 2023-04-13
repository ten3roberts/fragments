use std::{sync::Arc, task::Context};

use flax::{name, World};
use fragments_core::{effect::Executor, events::EventEmitter, frame::Frame, Widget};
use wgpu::{Color, CommandEncoderDescriptor, Operations, RenderPassDescriptor};
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{
    events::{on_redraw, on_resize},
    gpu::Gpu,
};

pub struct AppBuilder {}

impl AppBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(self) -> App {
        App {}
    }
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct App {}

impl App {
    /// Opens a window and enters the main event loop
    pub fn run(self, root: impl Widget + 'static) -> anyhow::Result<()> {
        // Create a new executor capable of executing the tasks in the UI
        let mut executor = Executor::new();

        // Contains the state
        let mut frame = Frame::new(World::new(), executor.spawner());

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop)?;

        frame.spawn_root(GpuProvider { window, root });

        let mut on_redraw = EventEmitter::new(on_redraw());
        let mut on_resize = EventEmitter::new(on_resize());
        event_loop.run(move |event, _, ctl| match event {
            Event::MainEventsCleared => {
                executor.update(&mut frame);
                on_redraw.emit(&frame.world, &());
            }
            Event::WindowEvent { window_id, event } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    *ctl = ControlFlow::Exit;
                }
                winit::event::WindowEvent::Resized(new_size) => {
                    on_resize.emit(&frame.world, &new_size);
                    tracing::info!("Resized");
                }
                _ => {}
            },
            _ => {}
        })
    }
}

struct GpuProvider<W> {
    window: Window,
    root: W,
}

impl<W> Widget for GpuProvider<W>
where
    W: 'static + Widget,
{
    fn render(self, scope: &mut fragments_core::Scope) {
        scope.set(name(), "GpuProvider".into());
        scope.use_future(
            async move {
                let gpu = Gpu::new(self.window).await;
                tracing::info!("Created gpu");
                Arc::new(gpu)
            },
            move |scope, gpu| {
                scope.on_global_event(
                    on_redraw(),
                    closure::closure!(clone gpu, |_, &()| {
                        let _span = tracing::debug_span!("Redrawing").entered();
                        let surface = gpu.surface.get_current_texture().unwrap();

                        let view = surface.texture.create_view(&Default::default());

                        let mut encoder =
                            gpu.device
                                .create_command_encoder(&CommandEncoderDescriptor {
                                    label: Some("on_redraw"),
                                });

                        {
                            let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                                label: None,
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: Operations {
                                        load: wgpu::LoadOp::Clear(Color::BLACK),
                                        store: true,
                                    },
                                })],
                                depth_stencil_attachment: None,
                            });
                        }

                        gpu.queue.submit([encoder.finish()]);
                        surface.present();

                    }),
                );
                scope.on_global_event(on_resize(), move |_, &new_size| {
                    tracing::info!("Rezing");
                    gpu.resize(new_size);
                });

                scope.attach(self.root);
            },
        )
    }
}
