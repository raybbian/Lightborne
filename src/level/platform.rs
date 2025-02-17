use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::shared::GroupLabel;

/// Component for moving platforms
#[derive(Default, Component)]
pub struct MovingPlatform;

