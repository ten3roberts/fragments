use flax::{component, Debuggable};
use glam::Vec2;

component! {
    /// The specified minimum width of a fragment
    pub min_width: f32 => [ Debuggable ],
    /// The specified minimum height of a fragment
    pub min_height: f32 => [ Debuggable ],

    /// The computed size of a fragment
    pub size: Vec2 => [ Debuggable ],
    /// The final placement of a fragment
    pub position: Vec2 => [ Debuggable ],
}
