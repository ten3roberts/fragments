use std::sync::Arc;

use flax::component;

use crate::{state::GraphicsState, WinitRequest};

component! {
    pub graphics_state: GraphicsState,
    pub winit_request: WinitRequest,
}
