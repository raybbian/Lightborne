use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

/// Labels used for rapier_2d [`CollisionGroups`]
pub struct GroupLabel;

impl GroupLabel {
    pub const PLAYER_COLLIDER: Group = Group::GROUP_1;
    pub const PLAYER_SENSOR: Group = Group::GROUP_2;
    pub const TERRAIN: Group = Group::GROUP_3;
    pub const LIGHT_RAY: Group = Group::GROUP_4;
    pub const LIGHT_SENSOR: Group = Group::GROUP_5;
    pub const HURT_BOX: Group = Group::GROUP_6;
    pub const WHITE_RAY: Group = Group::GROUP_7;
    pub const STRAND: Group = Group::GROUP_8;
    pub const BLUE_RAY: Group = Group::GROUP_9;
    pub const ALL: Group = Group::from_bits_truncate(!0);
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    Playing,
    Switching,
    Paused,
    StartMenu,
}

#[derive(Event, PartialEq, Eq)]
pub enum ResetLevel {
    Respawn,
    Switching,
}
