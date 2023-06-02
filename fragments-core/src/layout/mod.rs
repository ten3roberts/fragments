pub mod systems;

use flax::{component, Debuggable, Entity, EntityRef, World};
use glam::{vec2, Vec2};

use crate::components::ordered_children;

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

    pub layout: Layout => [ Debuggable ],
}

#[derive(Debug, Clone)]
pub enum Direction {
    Row,
    Column,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub dir: Direction,
}

impl Layout {
    fn update(
        &self,
        children: &[Entity],
        world: &World,
        parent_constraints: &Constraints,
    ) -> LayoutResult {
        let mut cursor = 0.0;

        let axis = match self.dir {
            Direction::Row => Vec2::X,
            Direction::Column => Vec2::Y,
        };

        let cross = axis.perp();

        let mut height = 0.0f32;

        let mut pending_margin = 0.0f32;

        let mut constraints = Constraints {
            min: Vec2::ZERO,
            max: parent_constraints.max,
        };

        for &child in children {
            let mut child = world.entity(child).expect("Invalid child");

            // Get a box that fits the child given the constraints
            let child_layout = update_layout(world, &mut child, &constraints);

            let m1 = vec2(child_layout.margin.right, child_layout.margin.top).dot(axis);
            let m2 = vec2(-child_layout.margin.left, -child_layout.margin.bottom).dot(axis);

            let front_margin = m1.max(m2);
            let back_margin = -m1.min(m2);

            assert!(front_margin >= 0.0, "{front_margin}");
            assert!(back_margin >= 0.0, "{back_margin}");

            // Collapse margins
            let margin = pending_margin.max(back_margin);
            // Step forward to the trailing margin before placing the child
            cursor += margin;

            tracing::info!(?front_margin, ?back_margin, ?margin);

            pending_margin = front_margin;

            *child.get_mut(size()).unwrap() = child_layout.size;
            *child.get_mut(local_position()).unwrap() = (cursor) * axis;

            cursor += child_layout.size.dot(axis);
            height = height.max(child_layout.size.dot(cross));
        }

        // Make sure to apply margin to the last child
        cursor += pending_margin;

        LayoutResult {
            size: (cursor * axis + height * cross)
                .abs()
                .clamp(parent_constraints.min, parent_constraints.max),
            margin: Margin {
                left: 5.0,
                right: 5.0,
                top: 5.0,
                bottom: 5.0,
            },
        }
    }
}

#[derive(Debug, Clone)]
struct Constraints {
    min: Vec2,
    max: Vec2,
}

#[derive(Debug)]
#[must_use]
struct LayoutResult {
    size: Vec2,
    margin: Margin,
}

#[derive(Debug, Clone, Copy)]
struct Margin {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

/// Update the size of the node given the constraints and return the size
fn update_layout(
    world: &World,
    entity: &mut EntityRef,
    parent_constraints: &Constraints,
) -> LayoutResult {
    // let _span = info_span!("update_layout", ?entity, ?constraints).entered();

    let mut res = LayoutResult {
        size: parent_constraints.min,
        margin: Margin {
            left: 10.0,
            right: 10.0,
            top: 10.0,
            bottom: 10.0,
        },
    };

    if let Ok(layout) = entity.get(layout()) {
        let children = entity.get(ordered_children());
        let children = children.as_ref().map(|v| v.as_slice()).unwrap_or_default();

        return layout.update(children, world, parent_constraints);
    }

    if let Ok(min_width) = entity.get(min_width()) {
        res.size.x = res.size.x.max(*min_width);
    }

    if let Ok(min_height) = entity.get(min_height()) {
        res.size.y = res.size.y.max(*min_height);
    }

    res
}
