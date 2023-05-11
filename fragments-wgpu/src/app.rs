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
    events::{on_frame, on_resize},
    gpu::Gpu,
    graphics::GpuProvider,
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

        frame.spawn_root(GpuProvider::new(window, root));

        let mut on_redraw = EventEmitter::new(on_frame());
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
                }
                _ => {}
            },
            _ => {}
        })
    }
}
