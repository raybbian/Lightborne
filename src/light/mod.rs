use bevy::{
    prelude::*,
    sprite::{AlphaMode2d, Material2dPlugin},
};

use enum_map::Enum;
use render::{LightMaterial, LightRenderData};
use segments::{
    cleanup_light_sources, simulate_light_sources, tick_light_sources, LightSegmentCache,
};
use sensor::{reset_light_sensors, update_light_sensors, HitByLightEvent};

use crate::{level::LevelSystems, shared::ResetLevel};

mod render;
pub mod segments;
pub mod sensor;

/// The speed of the light beam in units per [`FixedUpdate`].
const LIGHT_SPEED: f32 = 8.0;

/// The width of the rectangle used to represent [`LightSegment`](segments::LightSegmentBundle)s.
const LIGHT_SEGMENT_THICKNESS: f32 = 3.0;

/// [`Plugin`] that manages everything light related.
pub struct LightManagementPlugin;

impl Plugin for LightManagementPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<LightMaterial>::default())
            .init_resource::<LightRenderData>()
            .init_resource::<LightSegmentCache>()
            .add_event::<HitByLightEvent>()
            .add_systems(
                Update,
                (simulate_light_sources, update_light_sensors)
                    .chain()
                    .in_set(LevelSystems::Simulation),
            )
            .add_systems(
                FixedUpdate,
                (cleanup_light_sources, reset_light_sensors).run_if(on_event::<ResetLevel>),
            )
            .add_systems(
                FixedUpdate,
                tick_light_sources.in_set(LevelSystems::Simulation),
            );
    }
}

/// [`Enum`] for each of the light colors.
#[derive(Enum, Clone, Copy, Default, PartialEq, Debug, Eq, Hash)]
pub enum LightColor {
    #[default]
    Green,
    Red,
    White,
    Blue,
}

/// [`Color`] corresponding to each of the [`LightColor`]s. Note that the color values are greater
/// than 1.0 to take advantage of bloom.
impl From<LightColor> for Color {
    fn from(light_color: LightColor) -> Self {
        match light_color {
            LightColor::Red => Color::srgb(5.0, 0.0, 3.0),
            LightColor::Green => Color::srgb(3.0, 5.0, 0.0),
            LightColor::White => Color::srgb(2.0, 2.0, 2.0),
            LightColor::Blue => Color::srgb(1.0, 2.0, 4.0),
        }
    }
}

/// [`LightMaterial`] corresponding to each of the [`LightColor`]s.
impl From<LightColor> for LightMaterial {
    fn from(light_color: LightColor) -> Self {
        let color = Color::from(light_color);
        LightMaterial {
            color: color.into(),
            alpha_mode: AlphaMode2d::Blend,
            webgl_padding: Vec2::ZERO,
        }
    }
}

impl From<&String> for LightColor {
    fn from(value: &String) -> Self {
        match value.as_str() {
            "Red" => LightColor::Red,
            "Green" => LightColor::Green,
            "White" => LightColor::White,
            "Blue" => LightColor::Blue,
            _ => panic!("String {} does not represent Light Color", value),
        }
    }
}

impl LightColor {
    /// The number of bounces off of terrain each [`LightColor`] can make.
    pub fn num_bounces(&self) -> usize {
        match self {
            LightColor::Red => 2,
            _ => 1,
        }
    }

    pub fn lighting_color(&self) -> Vec3 {
        match self {
            LightColor::Red => Vec3::new(1.0, 0.1, 0.1),
            LightColor::Green => Vec3::new(0.0, 1.0, 0.0),
            LightColor::White => Vec3::new(0.8, 0.8, 0.5),
            LightColor::Blue => Vec3::new(0.0, 0.0, 1.0),
        }
    }
}

/// A [`Component`] marking the start of a light ray. These are spawned in
/// [`shoot_light`](crate::player::light::shoot_light), and simulated in
/// [`simulate_light_sources`]
#[derive(Component)]
#[require(Transform, Visibility, Sprite)]
pub struct LightRaySource {
    pub start_pos: Vec2,
    pub start_dir: Vec2,
    pub time_traveled: f32,
    pub color: LightColor,
    pub num_bounces: usize,
}
