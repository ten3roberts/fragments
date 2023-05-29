pub mod systems;

use flax::{component, Debuggable};
use glam::Vec2;

component! {
    /// The specified minimum width of a fragment
    pub min_width: f32 => [ Debuggable ],
    /// The specified minimum height of a fragment
    pub min_height: f32 => [ Debuggable ],

    /// The current computed size of a fragment
    pub size: Vec2 => [ Debuggable ],
    /// The final placement of a fragment on the canvas
    pub absolute_position: Vec2 => [ Debuggable ],
    /// The position relative the parent
    pub local_position: Vec2 => [ Debuggable ],
}
