use bevy::prelude::*;
use enum_map::{enum_map, EnumMap, Enum};

use crate::light::LightColor;

use super::{light::PlayerLightInventory, PlayerMarker};

impl FromWorld for LightIndicatorData {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh_handle = meshes
            .add(Circle::new(3.0))
            .into();

        let mut materials = world.resource_mut::<Assets<ColorMaterial>>();

        LightIndicatorData {
            mesh: mesh_handle,
            material_map: enum_map! {
                LightColor::Green => materials.add(ColorMaterial::from_color(LightColor::Green)).into(),
                LightColor::Red => materials.add(ColorMaterial::from_color(LightColor::Red)).into(),
                LightColor::White => materials.add(ColorMaterial::from_color(LightColor::White)).into(),
            },
            dimmed_material_map: enum_map! {
                LightDimColor::DimRed => materials.add(ColorMaterial::from_color(LightDimColor::DimRed)).into(),
                LightDimColor::DimGreen => materials.add(ColorMaterial::from_color(LightDimColor::DimGreen)).into(),
                LightDimColor::DimWhite => materials.add(ColorMaterial::from_color(LightDimColor::DimWhite)).into(),
            }
        }
    }
}

#[derive(Enum, Clone, Copy, Default, PartialEq, Debug)]
pub enum LightDimColor {
    #[default]
    DimGreen,
    DimRed,
    DimWhite,
}

impl From<LightColor> for LightDimColor {
    fn from(value: LightColor) -> Self {
        match value {
            LightColor::Green => LightDimColor::DimGreen,
            LightColor::Red => LightDimColor::DimRed, 
            LightColor::White => LightDimColor::DimWhite
        }
    }
}

impl From<LightDimColor> for Color {
    fn from(light_color: LightDimColor) -> Self {
        match light_color {
            LightDimColor::DimRed => Color::srgb(0.6, 0.0, 0.48),
            LightDimColor::DimGreen => Color::srgb(0.36, 0.6, 0.0),
            LightDimColor::DimWhite => Color::srgb(0.5, 0.5, 0.5),
        }
    }
}

impl FromWorld for LightDimIndicatorData {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh_handle = meshes
            .add(Circle::new(3.0))
            .into();

        let mut materials = world.resource_mut::<Assets<ColorMaterial>>();

        LightDimIndicatorData {
            mesh: mesh_handle,
            dimmed_material_map: enum_map! {
                LightDimColor::DimGreen => materials.add(ColorMaterial::from_color(LightDimColor::DimGreen)).into(),
                LightDimColor::DimRed => materials.add(ColorMaterial::from_color(LightDimColor::DimRed)).into(),
                LightDimColor::DimWhite => materials.add(ColorMaterial::from_color(LightDimColor::DimWhite)).into(),
            },
        }
    }
}

/// [`System`] that spawns the player's hurtbox [`Collider`] as a child entity.
// mut commands: Commands - needed for safely creating/removing data in the ECS World
pub fn add_light_indicator(mut commands: Commands, q_player: Query<Entity, Added<PlayerMarker>>, indicator_data:Res<LightIndicatorData>) {
    let Ok(player) = q_player.get_single() else {
        return;
    };

    let light_indicator = commands
        .spawn((
            indicator_data.mesh.clone(), 
            indicator_data.material_map[LightColor::Green].clone(),
            Visibility::Visible,
            Transform::from_xyz(-10.0, 10.0, 0.0),
            LightIndicatorMarker
        ))
        .id();

    commands.entity(player).add_child(light_indicator);
}

/// A resource that stored handles to the [`Mesh2d`] and [`MeshMaterial2d`] used in the rendering
/// of [`LightSegment`](super::segments::LightSegmentBundle)s.
#[derive(Resource)]
pub struct LightIndicatorData {
    pub mesh: Mesh2d,
    pub material_map: EnumMap<LightColor, MeshMaterial2d<ColorMaterial>>,
    pub dimmed_material_map: EnumMap<LightDimColor, MeshMaterial2d<ColorMaterial>>,
}

#[derive(Resource)]
pub struct LightDimIndicatorData {
    pub mesh: Mesh2d, 
    pub dimmed_material_map: EnumMap<LightDimColor, MeshMaterial2d<ColorMaterial>>,
}

#[derive(Default, Component)]
pub struct LightIndicatorMarker;

pub fn update_light_indicator(
    q_inventory: Query<&PlayerLightInventory>, 
    q_light_data: Query<Entity, With<LightIndicatorMarker>>,
    mut commands: Commands,
    light_data: Res<LightIndicatorData>
) {
    let Ok(indicator) = q_light_data.get_single() else {
        return;
    };

    let Ok(inventory) = q_inventory.get_single() else {
        return;
    };

    let material = match inventory.sources[inventory.current_color] {
        Some(_) => light_data.dimmed_material_map[inventory.current_color.into()].clone(),
        None => light_data.material_map[inventory.current_color].clone(),
    };

    commands.entity(indicator).insert(material);

    // commit -> pull req 
}