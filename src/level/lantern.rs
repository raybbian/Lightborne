use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use crate::lighting::LineLight2d;

#[derive(Component, Default)]
pub struct Lantern;

#[derive(Bundle, LdtkEntity)]
pub struct LdtkLanternBundle {
    #[sprite("lantern.png")]
    sprite: Sprite,
    #[with(lantern_light)]
    light: LineLight2d,
    #[default]
    lantern: Lantern,
}

#[derive(Bundle, LdtkEntity)]
pub struct LdtkLantern2Bundle {
    #[sprite("lantern2.png")]
    sprite: Sprite,
    #[with(lantern_light)]
    light: LineLight2d,
    #[default]
    lantern: Lantern,
}

pub fn lantern_light(_: &EntityInstance) -> LineLight2d {
    LineLight2d::point(Vec4::new(1.0, 0.8627, 0.7176, 1.0), 50.0, 0.02)
}
