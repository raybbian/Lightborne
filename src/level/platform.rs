use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::shared::GroupLabel;

struct Position {
    x: i32,
    y: i32
}

/// Component for platforms
#[derive(Default, Component)]
pub struct Platform {

}

/// Bundle for moving platforms
#[derive(Default, Bundle)]
pub struct MovingPlatformBundle {
    platform: Platform
}

fn spawn_platforms(query: Query<>) {

}


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