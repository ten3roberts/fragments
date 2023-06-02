use flax::{
    child_of,
    fetch::{entity_refs, EntityRefs},
    BoxedSystem, Component, Dfs, DfsBorrow, Entity, EntityQuery, EntityRef, Fetch, FetchExt, Opt,
    Query, QueryBorrow, System, World,
};
use glam::{vec2, Vec2};
use tracing::info_span;

use crate::{
    components::ordered_children,
    layout::{update_layout, Constraints},
};

use super::{absolute_position, local_position, min_height, min_width, size};

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

                // tracing::info!(?res, "Got layout result for tree {root:?}:");
            }
        })
        .boxed()
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
