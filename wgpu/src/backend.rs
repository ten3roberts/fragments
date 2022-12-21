use fragments::{components::resources, Backend, Widget};
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

pub struct WinitBackend<W> {
    pub root: W,
}

impl<W> Backend for WinitBackend<W>
where
    W: Widget,
{
    type Output = Result<()>;

    fn run(self, mut app: fragments::App) -> Self::Output {
        let event_loop = EventLoopBuilder::<WinitControl>::with_user_event().build();

        let request = WinitRequest {
            proxy: Mutex::new(event_loop.create_proxy()),
        };

        app.world_mut()
            .set(resources(), winit_request(), request)
            .unwrap();

        app.attach_root(self.root);

        event_loop.run(move |event, target, control_flow| match event {
            Event::UserEvent(control) => match control {
                WinitControl::OpenWindow(window, tx) => {
                    let window = window().build(target).map_err(Error::Window);
                    tx.send(window).ok();
                }
            },
            Event::WindowEvent { ref event, .. } => match event {
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
