use bevy::{
    prelude::*,
    sprite::{AlphaMode2d, Material2dPlugin},
};
use bevy_ecs_ldtk::prelude::*;

use enum_map::Enum;
use render::{LightMaterial, LightRenderData};
use segments::{
    cleanup_light_sources, simulate_light_sources, spawn_needed_segments, tick_light_sources,
    visually_sync_segments, LightSegmentCache, PrevLightBeamPlayback,
};

use crate::level::LevelSystems;

mod render;
pub mod segments;

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
            .register_ldtk_entity::<LightSegmentZBundle>("LightSegmentZMarker")
            .register_ldtk_entity::<LightSourceZBundle>("LightSourceZMarker")
            .add_systems(
                FixedUpdate,
                (
                    (
                        simulate_light_sources,
                        spawn_needed_segments,
                        visually_sync_segments,
                    )
                        .chain(),
                    tick_light_sources,
                )
                    .in_set(LevelSystems::Simulation),
            )
            // why does this need to be on update???
            .add_systems(Update, cleanup_light_sources.in_set(LevelSystems::Reset));
    }
}

#[derive(Default, Component)]
pub struct LightSourceZMarker;

#[derive(Bundle, LdtkEntity)]
pub struct LightSourceZBundle {
    #[default]
    marker: LightSourceZMarker,
    #[worldly]
    worldly: Worldly,
}

#[derive(Default, Component)]
pub struct LightSegmentZMarker;

#[derive(Bundle, LdtkEntity)]
pub struct LightSegmentZBundle {
    #[default]
    marker: LightSegmentZMarker,
    #[worldly]
    worldly: Worldly,
}

/// [`Enum`] for each of the light colors.
#[derive(Enum, Clone, Copy, Default, PartialEq, Debug, Eq, Hash)]
pub enum LightColor {
    #[default]
    Green,
    Purple,
    White,
    Blue,
}

/// [`LightMaterial`] corresponding to each of the [`LightColor`]s.
impl From<LightColor> for LightMaterial {
    fn from(light_color: LightColor) -> Self {
        let color = light_color.light_beam_color();
        LightMaterial {
            color: color.into(),
            alpha_mode: AlphaMode2d::Blend,
            _wasm_padding: Vec2::ZERO,
        }
    }
}

impl From<&String> for LightColor {
    fn from(value: &String) -> Self {
        match value.as_str() {
            "Purple" => LightColor::Purple,
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
            LightColor::Purple => 2,
            _ => 1,
        }
    }

    pub fn lighting_color(&self) -> Vec3 {
        match self {
            LightColor::Purple => Vec3::new(0.7, 0.2, 0.8),
            LightColor::Green => Vec3::new(0.0, 0.9, 0.5),
            LightColor::White => Vec3::new(0.8, 0.8, 0.5),
            LightColor::Blue => Vec3::new(0.1, 0.2, 0.8),
        }
    }

    pub fn light_beam_color(&self) -> Color {
        match self {
            LightColor::Purple => Color::srgb(1.5, 0.5, 3.0),
            LightColor::Green => Color::srgb(1.0, 4.0, 3.0),
            LightColor::White => Color::srgb(2.0, 2.0, 2.0),
            LightColor::Blue => Color::srgb(1.0, 2.0, 4.0),
        }
    }

    pub fn indicator_color(&self) -> Color {
        match self {
            LightColor::Purple => Color::srgb(0.7, 0.3, 1.0),
            LightColor::Green => Color::srgb(0.25, 0.9, 0.75),
            LightColor::White => Color::srgb(1.0, 1.0, 1.0),
            LightColor::Blue => Color::srgb(0.25, 0.5, 1.0),
        }
    }

    pub fn indicator_dimmed_color(&self) -> Color {
        self.indicator_color().with_alpha(0.15)
    }
}

/// A [`Component`] marking the start of a light ray. These are spawned in
/// [`shoot_light`](crate::player::light::shoot_light), and simulated in
/// [`simulate_light_sources`]
#[derive(Component)]
#[require(Transform, Visibility, Sprite, PrevLightBeamPlayback)]
pub struct LightBeamSource {
    pub start_pos: Vec2,
    pub start_dir: Vec2,
    pub time_traveled: f32,
    pub color: LightColor,
}
