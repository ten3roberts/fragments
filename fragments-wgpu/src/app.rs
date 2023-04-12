use std::task::Context;

use flax::World;
use fragments_core::{effect::Executor, frame::Frame, Widget};
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
    pub fn run(self) -> anyhow::Result<()> {
        // Create a new executor capable of executing the tasks in the UI
        let mut executor = Executor::new();

        // Contains the state
        let mut frame = Frame::new(World::new(), executor.spawner());

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop)?;

        frame.spawn_root(GpuProvider { window });

        event_loop.run(move |event, _, ctl| match event {
            Event::MainEventsCleared => {
                executor.update(&mut frame);
            }
            Event::WindowEvent { window_id, event } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    *ctl = ControlFlow::Exit;
                }
                winit::event::WindowEvent::Resized(new_size) => {
                    tracing::info!("Resized");
                }
                _ => {}
            },
            _ => {}
        })
    }
}

struct GpuProvider {
    window: Window,
}

impl Widget for GpuProvider {
    fn render(self, scope: &mut fragments_core::Scope) {
        scope.use_future(
            async move {
                let gpu = Gpu::new(self.window).await;
                tracing::info!("Created gpu");
                gpu
            },
            |scope, gpu| {
                scope.on_global_event(on_redraw(), |entity, &()| {
                    tracing::info!("Redrawing");
                });
                scope.on_global_event(on_resize(), move |_, &new_size| {
                    tracing::info!("Rezing");
                    gpu.resize(new_size);
                });
            },
        )
    }
}
