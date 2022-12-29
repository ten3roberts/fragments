use flax::component;
use fragments_core::context;

use crate::{state::GraphicsState, WindowManager};

component! {
    pub graphics_state: GraphicsState,
}

context! {
    pub window_manager: WindowManager,
}
