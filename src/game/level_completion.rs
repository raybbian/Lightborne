use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use crate::{game::Layers, ldtk::LdtkLevelParam, save::Save, ui::level_select::LevelProgress};

pub struct LevelCompletionPlugin;

impl Plugin for LevelCompletionPlugin {
    fn build(&self, app: &mut App) {
        app.register_ldtk_entity::<CompletionMarkerBundle>("StartMarker");
        app.register_ldtk_entity::<CompletionMarkerBundle>("EndMarker");
        app.insert_resource(InProgressLevel(LevelIid::default()));
    }
}

#[derive(Component)]
pub enum CompletionMarkerType {
    StartMarker,
    EndMarker,
}

#[derive(Bundle)]
pub struct CompletionMarkerBundle {
    marker_type: CompletionMarkerType,
    collider: Collider,
    sensor: Sensor,
    collision_groups: CollisionLayers,
}

#[derive(Resource)]
pub struct InProgressLevel(LevelIid);

impl LdtkEntity for CompletionMarkerBundle {
    fn bundle_entity(
        entity_instance: &EntityInstance,
        _: &LayerInstance,
        _: Option<&Handle<Image>>,
        _: Option<&TilesetDefinition>,
        _: &AssetServer,
        _: &mut Assets<TextureAtlasLayout>,
    ) -> Self {
        let marker_type = match entity_instance.identifier.as_ref() {
            "StartMarker" => CompletionMarkerType::StartMarker,
            "EndMarker" => CompletionMarkerType::EndMarker,
            _ => unreachable!(),
        };
        Self {
            marker_type,
            collider: Collider::rectangle(
                entity_instance.width as f32,
                entity_instance.height as f32,
            ),
            sensor: Sensor,
            collision_groups: CollisionLayers::new(Layers::SensorBox, Layers::PlayerHurtbox),
        }
    }
}

pub fn handle_start_end_markers(
    event: On<CollisionStart>,
    mut commands: Commands,
    q_completion_markers: Query<&CompletionMarkerType>,
    ldtk_level_param: LdtkLevelParam,
    mut res_in_progress_level: ResMut<InProgressLevel>,
    mut res_levels: ResMut<LevelProgress>,
) {
    let Ok(marker_type) = q_completion_markers.get(event.collider2) else {
        return;
    };
    match marker_type {
        CompletionMarkerType::StartMarker => {
            res_in_progress_level.0 = ldtk_level_param.cur_iid().expect("cur level exist");
        }
        CompletionMarkerType::EndMarker => {
            let current = ldtk_level_param.cur_iid().expect("cur level exist");
            if res_in_progress_level.0 != current {
                return;
            }
            let mut unlock_next = false;
            for level in res_levels.0.iter_mut() {
                if unlock_next {
                    level.locked = false;
                    commands.trigger(Save);
                    break;
                }
                if level.level_iid == current.to_string() {
                    level.complete = true;
                    unlock_next = true;
                }
            }
        }
    }
}
