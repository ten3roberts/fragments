use std::{borrow::Cow, sync::mpsc, time::Instant};

use futures::{channel::oneshot, Future};
use glam::{uvec2, UVec2};
use glfw::{Window, WindowEvent, WindowMode};
use slotmap::SlotMap;

use crate::{error::Error, error::Result};

pub struct GlfwBackend {}

impl Backend for GlfwBackend {
    type Output = Result<()>;

    fn run<W: fragments_core::Widget>(self, mut app: AppExecutor, root: W) -> Self::Output {
        let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS)?;

        let mut windows: SlotMap<WindowId, Window> = SlotMap::default();

        let (window, events) = glfw
            .create_window(800, 600, "Hello, World!", glfw::WindowMode::Windowed)
            .unwrap();

        let (control_tx, control_rx) = flume::unbounded();

        let window_control = WindowControl { tx: control_tx };

        // let root = app.attach_root(root);

        while !window.should_close() {
            glfw.poll_events();

            for control in control_rx.drain() {
                match control {
                    ControlEvent::CreateWindow { info, result } => {}
                    ControlEvent::CloseWindow(id) => {
                        if let Some(window) = windows.remove(id) {
                            window.close();
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

slotmap::new_key_type! {
    pub struct WindowId;
}

pub type WindowEvents = mpsc::Receiver<WindowEvent>;

#[derive(Debug)]
enum ControlEvent {
    CreateWindow {
        info: WindowInfo,
        result: oneshot::Sender<Result<(WindowId, WindowEvents)>>,
    },
    CloseWindow(WindowId),
}

pub struct WindowControl {
    tx: flume::Sender<ControlEvent>,
}

impl WindowControl {
    /// Create a new window
    pub fn create_window(
        &self,
        info: WindowInfo,
    ) -> impl Future<Output = Result<(WindowId, WindowEvents)>> {
        let (tx, rx) = oneshot::channel();

        let res = self
            .tx
            .send(ControlEvent::CreateWindow { info, result: tx })
            .map_err(|_| Error::NoBackend);

        async move {
            res?;
            let window = rx.await.map_err(|_| Error::NoBackend)??;

            Ok(window)
        }
    }

    pub fn close_window(&self, id: WindowId) {
        self.tx.send(ControlEvent::CloseWindow(id)).ok();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowInfo {
    title: Cow<'static, str>,
    width: u32,
    height: u32,
}

impl Default for WindowInfo {
    fn default() -> Self {
        Self {
            title: "Application Window".into(),
            width: 800,
            height: 600,
        }
    }
}

impl WindowInfo {
    /// Set the WindowInfo's title
    pub fn with_title(&mut self, title: impl Into<Cow<'static, str>>) -> &mut Self {
        self.title = title.into();
        self
    }
}

pub struct WindowManager {
    events: WindowEvents,
    on_resize: EventBroadcaster<UVec2>,
    on_frame: EventBroadcaster<()>,
    on_redraw: EventBroadcaster<()>,
}

impl WindowManager {
    pub fn update(&mut self, app: &mut App) {
        for event in self.events.try_iter() {
            tracing::info!(?event);
            match event {
                WindowEvent::Refresh => {
                    self.on_redraw.broadcast(app.world(), &());
                }
                WindowEvent::Size(width, height) => {
                    self.on_resize
                        .broadcast(app.world(), &uvec2(width as _, height as _));
                }
                _ => {}
            }
        }
    }
}
