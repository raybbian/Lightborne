use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use super::CollidableEntity;

#[derive(Component, Default)]
pub struct Door {
    pub is_open: bool,
    pub size: Vec2,
}

#[derive(Default, Bundle, LdtkEntity)]
pub struct DoorBundle {
    door: Door,
    #[sprite_sheet]
    sprite: Sprite,
    #[from_entity_instance]
    collider: CollidableEntity,
}