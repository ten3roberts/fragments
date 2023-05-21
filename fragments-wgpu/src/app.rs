use std::sync::Arc;

use flax::World;
use fragments_core::{effect::Executor, events::EventRegistry, frame::Frame, Widget};
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    events::{RedrawEvent, ResizeEvent},
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
        let mut frame = Frame::new(
            World::new(),
            executor.spawner(),
            Arc::new(EventRegistry::new()),
        );

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop)?;

        frame.spawn_root(GpuProvider::new(window, root));

        let events = frame.events.clone();
        event_loop.run(move |event, _, ctl| match event {
            Event::MainEventsCleared => {
                events.emit(&mut frame, &RedrawEvent);
                executor.update(&mut frame);
            }
            Event::WindowEvent { window_id, event } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    *ctl = ControlFlow::Exit;
                }
                winit::event::WindowEvent::Resized(new_size) => {
                    events.emit(&mut frame, &ResizeEvent(new_size));
                }
                _ => {}
            },
            _ => {}
        })
    }
}
