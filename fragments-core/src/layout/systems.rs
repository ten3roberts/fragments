use flax::{
    child_of,
    fetch::{entity_refs, EntityRefs},
    BoxedSystem, Component, Entity, EntityQuery, EntityRef, Fetch, FetchExt, Opt, Query,
    QueryBorrow, System, World,
};
use glam::{vec2, Vec2};

use crate::components::ordered_children;

use super::{min_width, position, size};

fn update_layout(world: &World, entity: &mut EntityRef, parent: &LayoutBox) -> LayoutResult {
    // Calculate the size required

    if let Ok(children) = entity.get(ordered_children()) {
        let mut current = LayoutBox {
            size: parent.size,
            pos: Vec2::ZERO,
        };

        let mut height = 0.0f32;

        for &child in &*children {
            let mut child = world.entity(child).expect("Invalid child");
            // Get the box that fits the child
            let child_layout = update_layout(world, &mut child, &current);

            *child.get_mut(position()).unwrap() = current.pos;
            *child.get_mut(size()).unwrap() = child_layout.size;

            current.size.x -= child_layout.size.x;
            current.pos.x += child_layout.size.x;

            height = height.max(child_layout.size.y);
        }

        LayoutResult {
            size: Vec2::new(current.pos.x, height),
        }
    } else {
        let min_width = entity.get(min_width()).as_deref().copied().unwrap_or(100.0);
        LayoutResult {
            size: vec2(min_width, 100.0),
        }
    }
}

// /// Updates the layout under `root` using the entity's current size and position.
// fn update_layout_root(world: &World, root: EntityRef, size: Vec2, pos: Vec2) {
//     let layout_box = LayoutBox { size, pos };

//     let res = update_layout(world, entity, &layout_box);
//     tracing::info!(?res, "Got layout result for tree {root}:");
// }

pub fn update_layout_system() -> BoxedSystem {
    System::builder()
        .read()
        .with(Query::new((entity_refs(), position(), size())).without_relation(child_of))
        .build(
            |world: &World,
             mut roots: QueryBorrow<(EntityRefs, Component<Vec2>, Component<Vec2>), _>| {
                for (mut root, &pos, &size) in &mut roots {
                    let res = update_layout(world, &mut root, &LayoutBox { size, pos });

                    tracing::info!(?res, "Got layout result for tree {root:?}:");
                }
            },
        )
        .boxed()
}

#[derive(Debug)]
struct LayoutResult {
    size: Vec2,
}

#[derive(Debug)]
struct LayoutBox {
    pos: Vec2,
    size: Vec2,
}

#[derive(Fetch)]
struct ChildQuery {
    children: Opt<Component<Vec<Entity>>>,

    min_width: Opt<Component<f32>>,
}

impl ChildQuery {
    fn new() -> Self {
        Self {
            children: ordered_children().opt(),
            min_width: min_width().opt(),
        }
    }
}
