use std::sync::Arc;

use flax::{component, Debuggable};

use crate::{gpu::Gpu, graphics::shader::Shader, mesh::Mesh};

component! {
    pub(crate) graphics_state: Gpu,
    pub(crate) mesh: Arc<Mesh>,
    pub(crate) shader: Arc<Shader>,



}

component! {
    pub rectangle: () => [ Debuggable ],
}
