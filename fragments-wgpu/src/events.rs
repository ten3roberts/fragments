use flax::component;
use fragments_core::events::EventHandler;
use winit::{dpi::PhysicalSize, event::KeyboardInput};

component! {
    pub on_redraw: EventHandler<()>,
    pub on_frame: EventHandler<()>,
    pub on_resize: EventHandler<PhysicalSize<u32>>,
    pub on_keyboard_input: EventHandler<KeyboardInput>,
}
