use bevy::prelude::*;
use enum_map::{enum_map, EnumMap};

use crate::{camera::HIGHRES_LAYER, level::LevelSystems, light::LightColor, player::PlayerMarker};

use super::PlayerLightInventory;

pub struct LightIndicatorPlugin;

impl Plugin for LightIndicatorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightIndicatorData>()
            .add_systems(
                PreUpdate,
                add_light_indicator.in_set(LevelSystems::Processing),
            )
            .add_systems(FixedUpdate, update_light_indicator);
    }
}

/// A resource that stored handles to the [`Mesh2d`] and [`MeshMaterial2d`] used in the rendering
/// of [`LightSegment`](super::segments::LightSegmentBundle)s.
#[derive(Resource)]
pub struct LightIndicatorData {
    pub mesh: Mesh2d,
    pub material_map: EnumMap<LightColor, MeshMaterial2d<ColorMaterial>>,
    pub dimmed_material_map: EnumMap<LightColor, MeshMaterial2d<ColorMaterial>>,
}

#[derive(Default, Component)]
pub struct LightIndicatorMarker;

impl FromWorld for LightIndicatorData {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh_handle = meshes.add(Circle::new(3.0)).into();

        let mut materials = world.resource_mut::<Assets<ColorMaterial>>();

        LightIndicatorData {
            mesh: mesh_handle,
            material_map: enum_map! {
                val => materials.add(val.indicator_color()).into(),
            },
            dimmed_material_map: enum_map! {
                val => materials.add(val.indicator_dimmed_color()).into(),
            },
        }
    }
}

/// [`System`] that spawns the player's hurtbox [`Collider`] as a child entity.
// mut commands: Commands - needed for safely creating/removing data in the ECS World
pub fn add_light_indicator(
    mut commands: Commands,
    q_player: Query<Entity, Added<PlayerMarker>>,
    indicator_data: Res<LightIndicatorData>,
) {
    let Ok(player) = q_player.get_single() else {
        return;
    };

    let light_indicator = commands
        .spawn((
            indicator_data.mesh.clone(),
            indicator_data.material_map[LightColor::Green].clone(),
            Visibility::Visible,
            Transform::from_xyz(-10.0, 10.0, 0.0),
            LightIndicatorMarker,
            HIGHRES_LAYER,
        ))
        .id();

    commands.entity(player).add_child(light_indicator);
}

pub fn update_light_indicator(
    q_inventory: Query<&PlayerLightInventory>,
    q_light_data: Query<Entity, With<LightIndicatorMarker>>,
    mut commands: Commands,
    light_data: Res<LightIndicatorData>,
) {
    let Ok(indicator) = q_light_data.get_single() else {
        return;
    };

    let Ok(inventory) = q_inventory.get_single() else {
        return;
    };

    match inventory.current_color {
        None => commands.entity(indicator).insert(Visibility::Hidden),
        Some(_) => commands.entity(indicator).insert(Visibility::Visible),
    };

    if let Some(color) = inventory.current_color {
        let material = match inventory.sources[color] {
            false => light_data.dimmed_material_map[color].clone(),
            true => light_data.material_map[color].clone(),
        };

        commands.entity(indicator).insert(material);
    }
}
