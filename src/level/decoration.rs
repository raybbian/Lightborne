use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use crate::lighting::LineLight2d;

pub struct DecorationPlugin;

impl Plugin for DecorationPlugin {
    fn build(&self, app: &mut App) {
        app.register_ldtk_entity::<LdtkTreeBranchBundle>("Treebranch")
            .register_ldtk_entity::<LdtkLanternBundle>("Lantern")
            .register_ldtk_entity::<LdtkLantern2Bundle>("Lantern2");
    }
}

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

#[derive(Component, Default)]
pub struct TreeBranch;

#[derive(Bundle, LdtkEntity)]
pub struct LdtkTreeBranchBundle {
    #[sprite_sheet]
    sprite: Sprite,
    #[default]
    lantern: TreeBranch,
}
