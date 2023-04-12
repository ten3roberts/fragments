use fragments_core::{
    components::resources, effect::AppExecutor, events::EventBroadcaster, Backend, Scope, Widget,
};
use futures::{channel::oneshot, Future, FutureExt};
use parking_lot::Mutex;
use tracing::info_span;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::{Window, WindowBuilder},
};

use crate::{
    error::{Error, Result},
    events::{on_frame, on_redraw, on_resize},
    window_manager,
};

enum ControlEvent {
    OpenWindow(
        Box<dyn FnOnce() -> WindowBuilder + Send + Sync>,
        oneshot::Sender<Result<Window>>,
    ),
}

/// Allow communication with the winit backend to E.g; open windows
pub struct WindowManager {
    proxy: Mutex<EventLoopProxy<ControlEvent>>,
}

impl WindowManager {
    pub fn create_window<F>(&self, window: F) -> impl Future<Output = Result<Window>>
    where
        F: FnOnce() -> WindowBuilder + Send + Sync + 'static,
    {
        let (tx, rx) = oneshot::channel();

        self.proxy
            .lock()
            .send_event(ControlEvent::OpenWindow(Box::new(window), tx))
            .ok()
            .expect("Failed to request window");

        rx.map(|v| v.unwrap())
    }
}

pub struct WinitBackend {}

impl Backend for WinitBackend {
    type Output = Result<()>;

    fn run<W: Widget>(self, mut app: AppExecutor, root: W) -> Self::Output {
        let event_loop = EventLoopBuilder::<ControlEvent>::with_user_event().build();

        let wmanager = WindowManager {
            proxy: Mutex::new(event_loop.create_proxy()),
        };

        let root = app.attach_root(|scope: &mut Scope| {
            scope.provide_context(window_manager(), wmanager);

            scope.attach_child(root);
        });

        let _span = info_span!("event_loop").entered();

        let mut on_resize = EventBroadcaster::new(on_resize());
        let mut on_redraw = EventBroadcaster::new(on_redraw());
        let mut on_frame = EventBroadcaster::new(on_frame());

        event_loop.run(move |event, target, control_flow| match event {
            Event::UserEvent(control) => match control {
                ControlEvent::OpenWindow(window, tx) => {
                    tracing::info!("Got window request");
                    let window = window().build(target).map_err(Error::Window);
                    tracing::info!("Opened window");

                    tx.send(window).unwrap();
                }
            },
            Event::MainEventsCleared => {
                on_frame.broadcast(app.world(), &());
                app.update();
            }
            Event::RedrawRequested(_window) => {
                on_redraw.broadcast(app.world(), &());
            }
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(new_size) => {
                    on_resize.broadcast(app.world(), new_size);
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => {}
            },
            _ => {}
        });
    }
}
