use std::default;

use bevy::{prelude::*, text::cosmic_text::ttf_parser::loca, utils::hashbrown::hash_map::IterMut};
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy::log::LogPlugin;

use crate::{player::{PlayerBundle, PlayerMarker}, shared::{GroupLabel, ResetLevel}};

use super::LevelSystems;


/// Plugin for handling platforms
pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlatformToggleEvent>()
            .add_systems(PreUpdate,(initialize_platforms,).in_set(LevelSystems::Processing).run_if(on_event::<ResetLevel>))
            .add_systems(Update, on_platform_changed.in_set(LevelSystems::Simulation).run_if(on_event::<ResetLevel>));
            //.add_systems(FixedUpdate, reset_platforms.run_if(on_event::<ResetLevel>)); // May be unnecessary if resetting on loading and resetting the same way
    }
}

/// Component for moving platforms
#[derive(Default, Component)]
pub struct MovingPlatform {
    pub path: Vec<IVec2>,
    pub default_state: bool,
    pub speed: f32,
}

/// Bundle for moving platforms
#[derive(Default, Bundle, LdtkEntity)]
pub struct MovingPlatformBundle {
    pub platform: MovingPlatform,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    #[sprite("platform.png")]
    pub sprite: Sprite
}

/// Event that will toggle the movement of a platform
#[derive(Event)]
pub struct PlatformToggleEvent {
    pub state: bool
}

pub fn initialize_platforms(
    mut commands: Commands,
    level_q: Query<(&MovingPlatform, &Transform, &GlobalTransform)>,
    player_q: Query<&GlobalTransform, With<PlayerMarker>>,
    asset_server: Res<AssetServer>
) {
    println!("Hello world!");
    for (platform, local_transform, global_transform) in level_q.iter() {
        println!("Entity found!");
        let speed = platform.speed;
        let path = (&platform.path).to_vec();
        let default_state = platform.default_state;
        let x = global_transform.translation().x;
        let y = global_transform.translation().y;
        println!("Current Position: x = {}, y = {}", x, y);
        let Ok(player) = player_q.get_single() else {
            return;
        };
        let xP = player.translation().x;
        let yP = player.translation().y;
        println!("Current Player Position: x = {}, y = {}", xP, yP);
        /*
        let platform_texture = asset_server.load("platform.png");
        commands.spawn(
            MovingPlatformBundle {
                platform: MovingPlatform {
                    speed,
                    path,
                    default_state,
                    id
                },
                sprite: Sprite {
                    color: Color::rgb(0.0, 1.0, 0.0),
                    flip_x: false,
                    flip_y: false,
                    custom_size: Default::default(),
                    rect: None,
                    image: platform_texture,
                    anchor: bevy::sprite::Anchor::Center,
                    texture_atlas: Default::default(),
                    image_mode: Default::default()
                },
                transform: Transform::default(),
                global_transform: GlobalTransform::default()
            }
        );
        */
    }
}

pub fn on_platform_changed() {
    println!("Not implemented yet!")
}

//pub fn reset_platforms() {
//
//}





/*

Platforms have:
    * Position -> (x,y)
    * Path -> Vec<>
    * Speed -> (x)
    * Triggered/Continuous -> enum(Triggered/Continous)
    * Repeat -> boolean
    * DefaultActive
    * IsActive
*/