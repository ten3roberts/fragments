use std::{cell::RefCell, rc::Rc, sync::Arc};

use flax::{BoxedSystem, Entity, Query, QueryBorrow, Schedule, System, World};
use fragments_core::{
    effect::{Executor, FutureEffect},
    events::EventRegistry,
    frame::Frame,
    Widget,
};
use glam::Mat4;
use winit::{
    dpi::PhysicalSize,
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    events::ResizeEvent,
    gpu::Gpu,
    graphics::{proj_matrix, scale_to_window, view_matrix},
    renderer::Renderer,
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

        let events = frame.events.clone();

        // Initialize the gpu and create a camera entity and renderer

        let gpu = futures::executor::block_on(Gpu::new(window));

        let gpu = Arc::new(gpu);

        let camera = Entity::builder()
            .set(view_matrix(), Mat4::IDENTITY)
            .set(proj_matrix(), Mat4::IDENTITY)
            .tag(scale_to_window())
            .spawn(&mut frame.world);

        let mut renderer = Renderer::new(&gpu, camera);

        frame.spawn_root(root);

        let mut on_resized = Schedule::new().with_system(resize_cameras_system());

        event_loop.run(move |event, _, ctl| match event {
            Event::MainEventsCleared => {
                // Update the UI
                executor.update(&mut frame);
                if let Err(err) = draw(&mut frame, &mut renderer) {
                    tracing::error!("Error drawing: {:?}", err);
                }
            }
            Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    *ctl = ControlFlow::Exit;
                }
                winit::event::WindowEvent::Resized(mut new_size) => {
                    if let Err(err) = on_resized.execute_seq_with(&mut frame.world, &mut new_size) {
                        tracing::error!("Error resizing: {:?}", err);
                    };

                    renderer.resize(new_size);

                    events.emit(&mut frame, &ResizeEvent(new_size));
                }
                _ => {}
            },
            _ => {}
        })
    }
}

fn draw(frame: &mut Frame, renderer: &mut Renderer) -> anyhow::Result<()> {
    renderer.update(&mut frame.world)?;
    renderer.draw()?;
    Ok(())
}

fn resize_cameras_system() -> BoxedSystem<PhysicalSize<u32>> {
    System::builder_with_data()
        .with(Query::new(proj_matrix().as_mut()).with(scale_to_window()))
        .read_context()
        .build(|mut query: QueryBorrow<_, _>, size: &PhysicalSize<u32>| {
            query.for_each(|proj: &mut Mat4| {
                *proj =
                    Mat4::orthographic_lh(0.0, size.width as _, size.height as _, 0.0, 0.0, 100.0);
            });
        })
        .boxed()
}
