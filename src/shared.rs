use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

pub const LYRA_RESPAWN_EPSILON: f32 = 3.0;

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
    pub const CRYSTAL_SHARD: Group = Group::GROUP_10;
    pub const PLATFORM: Group = Group::GROUP_11;
    pub const BLACK_RAY: Group = Group::GROUP_12;
    pub const ALL: Group = Group::from_bits_truncate(!0);
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    Playing,
    Animating,
    Paused,
    Ui,
}

#[derive(SubStates, Default, Debug, Clone, PartialEq, Eq, Hash)]
#[source(GameState = GameState::Animating)]
pub enum AnimationState {
    #[default]
    Switch,
    Respawn,
    Shard,
    ShardDialogue, // FIXME: copied to shit LOL
    Cruciera,
    CrucieraDialogue,
}

#[derive(SubStates, Default, Debug, Clone, PartialEq, Eq, Hash)]
#[source(GameState = GameState::Ui)]
pub enum UiState {
    #[default]
    None,
    LevelSelect,
    Settings,
    StartMenu,
}

#[derive(Event, PartialEq, Eq)]
pub enum ResetLevel {
    /// Sent to run systems that reset the player state on respawn. If you are trying to kill the
    /// player, use `KillPlayerEvent` instead
    Respawn,
    /// Sent to run systems that reset the level state on level switch
    Switching,
}
