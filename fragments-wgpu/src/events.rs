use flax::component;
use fragments_core::{context, events::EventHandler};
use winit::{dpi::PhysicalSize, event::KeyboardInput};

pub struct RedrawEvent;
pub struct ResizeEvent(pub PhysicalSize<u32>);

context! {
    pub(crate) window_size:PhysicalSize<u32>,
}
