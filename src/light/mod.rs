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

use crate::{level::LevelSystems, lighting::LineLight2d};

mod render;
pub mod segments;

/// The speed of the light beam in units per [`FixedUpdate`].
const LIGHT_SPEED: f32 = 8.0;
const BLOCK_WIDTH: f32 = 8.0;

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
            .register_ldtk_entity::<LightSourceBundle>("LightSource")
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
            .add_systems(Update, cleanup_light_sources.in_set(LevelSystems::Reset))
            .add_systems(
                PostUpdate,
                spawn_level_light_beams.in_set(LevelSystems::Simulation),
            )
            .add_systems(
                PostUpdate,
                add_light_beam_added.in_set(LevelSystems::Processing),
            );
    }
}

fn add_light_beam_added(
    ldtk_sources: Query<
        (Entity, &LightBeamLDTKSource, &GlobalTransform),
        Added<LightBeamLDTKSource>,
    >,
    mut commands: Commands,
) {
    for source in ldtk_sources.iter() {
        commands.entity(source.0).insert(LightBeamSourceAdded);
    }
}

fn spawn_level_light_beams(
    mut commands: Commands,
    ldtk_sources: Query<
        (Entity, &LightBeamLDTKSource, &GlobalTransform),
        Added<LightBeamSourceAdded>,
    >,
) {
    for (_, source, transform) in ldtk_sources.iter() {
        let ray_dir_int = source.direction - source.position;
        let ray_dir = Vec2::new(
            ray_dir_int.x as f32 + (source.x_offset / 8.0),
            -(ray_dir_int.y as f32 + (source.y_offset / 8.0)),
        )
        .normalize();
        let ray_pos = Vec2::new(
            transform.translation().truncate().x + source.x_offset,
            transform.translation().truncate().y + source.y_offset,
        );
        let shoot_color = LightColor::Black;

        /* Used for the source image; currently not used
        let mut source_transform = Transform::from_translation(ray_pos.extend(light_source_z.translation.z));
        source_transform.rotate_z(ray_dir.to_angle());
        let mut source_sprite = Sprite::from_image(asset_server.load("light/compass.png"));
        source_sprite.color = Color::srgb(2.0, 2.0, 2.0);
        let mut outer_source_sprite = Sprite::from_image(asset_server.load("light/compass-gold.png"));
        outer_source_sprite.color = shoot_color.light_beam_color().mix(&Color::BLACK, 0.4);
        */

        let light_beam_source = LightBeamSource {
            start_pos: ray_pos,
            start_dir: ray_dir,
            time_traveled: 1000.0,
            color: shoot_color,
        };
        commands
            .spawn(light_beam_source)
            .insert(PrevLightBeamPlayback::default())
            .insert(LineLight2d::point(
                shoot_color.lighting_color().extend(1.0),
                30.0,
                0.0,
            ));
    }
}

#[derive(Default, Component)]
pub struct BlackRayComponent;

#[derive(Default, Component)]
pub struct LightBeamSourceAdded;

#[derive(Default, Component)]
pub struct LightSourceZMarker;

#[derive(Bundle, LdtkEntity)]
pub struct LightSourceZBundle {
    #[default]
    marker: LightSourceZMarker,
    #[worldly]
    worldly: Worldly,
}

// Bundle for LDTK Light Source
#[derive(Bundle, LdtkEntity)]
pub struct LightSourceBundle {
    #[from_entity_instance]
    pub beam_source: LightBeamLDTKSource,
}

// Component for LDTK Light Source
#[derive(Default, Component)]
pub struct LightBeamLDTKSource {
    pub direction: IVec2,
    pub position: IVec2,
    pub x_offset: f32,
    pub y_offset: f32,
}

impl From<&bevy_ecs_ldtk::EntityInstance> for LightBeamLDTKSource {
    fn from(entity_instance: &bevy_ecs_ldtk::EntityInstance) -> Self {
        let height = entity_instance.height;
        let position = IVec2::new(
            entity_instance.grid.x,
            entity_instance.grid.y + (height / (BLOCK_WIDTH as i32)) - 1,
        );
        let x_offset = *entity_instance.get_float_field("XOffset").unwrap();
        let y_offset = *entity_instance.get_float_field("YOffset").unwrap();
        let direction = *entity_instance.get_point_field("Direction").unwrap();
        LightBeamLDTKSource {
            direction,
            position,
            x_offset,
            y_offset,
        }
    }
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
    Black,
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
            "Black" => LightColor::Black,
            _ => panic!("String {} does not represent Light Color", value),
        }
    }
}

impl LightColor {
    /// The number of bounces off of terrain each [`LightColor`] can make.
    pub fn num_bounces(&self) -> usize {
        match self {
            LightColor::Purple => 2,
            LightColor::Black => 0,
            _ => 1,
        }
    }

    pub fn lighting_color(&self) -> Vec3 {
        match self {
            LightColor::Purple => Vec3::new(0.7, 0.2, 0.8),
            LightColor::Green => Vec3::new(0.0, 0.9, 0.5),
            LightColor::White => Vec3::new(0.8, 0.8, 0.5),
            LightColor::Blue => Vec3::new(0.1, 0.2, 0.8),
            LightColor::Black => Vec3::new(0.2, 0.2, 0.2),
        }
    }

    pub fn light_beam_color(&self) -> Color {
        match self {
            LightColor::Purple => Color::srgb(1.5, 0.5, 3.0),
            LightColor::Green => Color::srgb(1.0, 4.0, 3.0),
            LightColor::White => Color::srgb(2.0, 2.0, 2.0),
            LightColor::Blue => Color::srgb(1.0, 2.0, 4.0),
            LightColor::Black => Color::srgb(0.2, 0.2, 0.2),
        }
    }

    pub fn indicator_color(&self) -> Color {
        match self {
            LightColor::Purple => Color::srgb(0.7, 0.3, 1.0),
            LightColor::Green => Color::srgb(0.25, 0.9, 0.75),
            LightColor::White => Color::srgb(1.0, 1.0, 1.0),
            LightColor::Blue => Color::srgb(0.25, 0.5, 1.0),
            LightColor::Black => Color::srgb(0.2, 0.2, 0.2),
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
