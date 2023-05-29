use flax::{
    child_of,
    fetch::{entity_refs, EntityRefs},
    BoxedSystem, Component, Dfs, DfsBorrow, Entity, EntityQuery, EntityRef, Fetch, FetchExt, Opt,
    Query, QueryBorrow, System, World,
};
use glam::{vec2, Vec2};
use tracing::info_span;

use crate::components::ordered_children;

use super::{absolute_position, local_position, min_height, min_width, size};

/// Update the size of the node given the constraints and return the size
fn update_layout(world: &World, entity: &mut EntityRef, constraints: &Constraints) -> LayoutResult {
    // let _span = info_span!("update_layout", ?entity, ?constraints).entered();

    let mut res = LayoutResult {
        size: constraints.min,
    };

    if let Ok(children) = entity.get(ordered_children()) {
        let mut constraints = Constraints {
            min: Vec2::ZERO,
            max: constraints.max,
        };

        let mut cursor = Vec2::ZERO;
        let mut line_height = 0.0f32;

        for &child in &*children {
            let mut child = world.entity(child).expect("Invalid child");
            // Get the box that fits the child
            let child_layout = update_layout(world, &mut child, &constraints);

            *child.get_mut(local_position()).unwrap() = cursor;

            cursor.x += child_layout.size.x + 5.0;
            line_height = line_height.max(child_layout.size.y);

            constraints.max.x -= child_layout.size.x;
        }

        cursor.y += line_height + 50.0;

        res.size = cursor;
    }

    if let Ok(min_width) = entity.get(min_width()) {
        res.size.x = res.size.x.max(*min_width);
    }

    if let Ok(min_height) = entity.get(min_height()) {
        res.size.y = res.size.y.max(*min_height);
    }

    *entity.get_mut(size()).unwrap() = res.size;

    res
}

pub fn update_transform_system() -> BoxedSystem {
    System::builder()
        .with(
            Query::new((local_position(), absolute_position().as_mut()))
                .with_strategy(Dfs::new(child_of)),
        )
        .build(|mut q: DfsBorrow<_>| {
            q.traverse(
                &Vec2::ZERO,
                |(local_pos, pos): (&Vec2, &mut Vec2), _, parent_pos| {
                    *pos = *parent_pos + *local_pos;
                    *pos
                },
            );
        })
        .boxed()
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
        .with(
            Query::new(entity_refs())
                .with(size())
                .without_relation(child_of),
        )
        .build(|world: &World, mut roots: QueryBorrow<EntityRefs, _>| {
            for mut root in &mut roots {
                let size = *root.get(size()).unwrap();

                let res = update_layout(
                    world,
                    &mut root,
                    &Constraints {
                        min: Vec2::ZERO,
                        max: size,
                    },
                );

                tracing::info!(?res, "Got layout result for tree {root:?}:");
            }
        })
        .boxed()
}

#[derive(Debug, Clone)]
struct Constraints {
    min: Vec2,
    max: Vec2,
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
