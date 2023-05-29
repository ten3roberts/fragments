use std::sync::Arc;

use flax::{
    component, BoxedSystem, Debuggable, Entity, EntityBorrow, Mutable, Query, QueryBorrow,
    Schedule, System, World,
};
use fragments_core::{
    effect::Executor,
    events::EventRegistry,
    frame::Frame,
    layout::{
        absolute_position, local_position, size,
        systems::{update_layout_system, update_transform_system},
    },
    Widget,
};
use glam::{vec2, Mat4, Vec2};
use winit::{
    dpi::PhysicalSize,
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    events::{RedrawEvent, ResizeEvent},
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

component! {
    gpu: Arc<Gpu> => [ Debuggable ],
    renderer: Renderer,

    state,
}

struct Canvas<W> {
    size: Vec2,
    root: W,
}

impl<W> Widget for Canvas<W>
where
    W: Widget,
{
    fn mount(self, scope: &mut fragments_core::Scope<'_>) {
        scope.set(size(), self.size);
        scope.set(absolute_position(), Vec2::ZERO);
        scope.set(local_position(), Vec2::ZERO);
        scope.attach(self.root);
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
        let window_size = window.inner_size();

        let events = frame.events.clone();

        // Initialize the gpu and create a camera entity and renderer

        let gpu = futures::executor::block_on(Gpu::new(window));

        let gpu = Arc::new(gpu);

        let camera = Entity::builder()
            .set(view_matrix(), Mat4::IDENTITY)
            .set(proj_matrix(), Mat4::IDENTITY)
            .tag(scale_to_window())
            .spawn(&mut frame.world);

        let renderer = Renderer::new(&gpu, camera);

        Entity::builder()
            .set(self::gpu(), gpu)
            .set(self::renderer(), renderer)
            .append_to(&mut frame.world, state())
            .unwrap();

        frame.spawn_root(Canvas {
            size: vec2(window_size.width as f32, window_size.height as f32),
            root,
        });

        let mut on_resized = Schedule::new()
            .with_system(resize_cameras_system())
            .with_system(resize_renderer_system());

        let mut on_frame = Schedule::new()
            .with_system(update_layout_system())
            .with_system(update_transform_system())
            .with_system(draw_system());

        event_loop.run(move |event, _, ctl| match event {
            Event::MainEventsCleared => {
                // Update the UI
                events.emit(&mut frame, &RedrawEvent);
                executor.update(&mut frame);
                if let Err(err) = on_frame.execute_seq(&mut frame.world) {
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

                    events.emit(&mut frame, &ResizeEvent(new_size));
                }
                _ => {}
            },
            _ => {}
        })
    }
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

fn resize_renderer_system() -> BoxedSystem<PhysicalSize<u32>> {
    System::builder_with_data()
        .with(Query::new(renderer().as_mut()))
        .read_context()
        .build(|mut query: QueryBorrow<_>, size: &PhysicalSize<u32>| {
            query.for_each(|renderer: &mut Renderer| {
                renderer.resize(*size);
            })
        })
        .boxed()
}

fn draw_system() -> BoxedSystem {
    System::builder()
        .read()
        .with(Query::new(renderer().as_mut()).entity(state()))
        .build(
            |world: &World, mut query: EntityBorrow<Mutable<Renderer>>| -> anyhow::Result<()> {
                if let Ok(renderer) = query.get() {
                    renderer.update(world)?;
                    renderer.draw()?;
                }

                Ok(())
            },
        )
        .boxed()
}
