use fragments::{components::resources, effect::AppExecutor, events::send_event, Backend, Widget};
use futures::{channel::oneshot, Future, FutureExt};
use parking_lot::Mutex;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::{Window, WindowBuilder},
};

use crate::{
    components::winit_request,
    error::{Error, Result},
    events::{on_frame, on_redraw, on_resize},
};

enum WinitControl {
    OpenWindow(
        Box<dyn FnOnce() -> WindowBuilder + Send + Sync>,
        oneshot::Sender<Result<Window>>,
    ),
}

pub struct WinitRequest {
    proxy: Mutex<EventLoopProxy<WinitControl>>,
}

impl WinitRequest {
    pub fn request_window<F: FnOnce() -> WindowBuilder + Send + Sync + 'static>(
        &self,
        window: F,
    ) -> impl Future<Output = Result<Window>> {
        let (tx, rx) = oneshot::channel();

        self.proxy
            .lock()
            .send_event(WinitControl::OpenWindow(Box::new(window), tx))
            .ok()
            .expect("Failed to request window");

        rx.map(|v| v.unwrap())
    }
}

pub struct WinitBackend {}

impl Backend for WinitBackend {
    type Output = Result<()>;

    fn run<W: Widget>(self, mut app: AppExecutor, root: W) -> Self::Output {
        let event_loop = EventLoopBuilder::<WinitControl>::with_user_event().build();

        let request = WinitRequest {
            proxy: Mutex::new(event_loop.create_proxy()),
        };

        app.world_mut()
            .set(resources(), winit_request(), request)
            .unwrap();

        let root = app.attach_root(root);

        tracing::info!("Entering event loop");
        event_loop.run(move |event, target, control_flow| match event {
            Event::UserEvent(control) => match control {
                WinitControl::OpenWindow(window, tx) => {
                    tracing::info!("Got window request");
                    let window = window().build(target).map_err(Error::Window);
                    tracing::info!("Opened window");

                    tx.send(window).unwrap();
                }
            },
            Event::MainEventsCleared => {
                send_event(app.world(), root, on_frame(), &());
                app.update();
            }
            Event::RedrawRequested(_window) => {
                send_event(app.world(), root, on_redraw(), &());
            }
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(new_size) => {
                    send_event(app.world(), root, on_resize(), new_size);
                }
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,
                _ => {}
            },
            _ => {}
        });
    }
}
