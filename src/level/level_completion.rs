use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{player::PlayerHurtMarker, shared::GroupLabel, ui::level_select::Levels};

use super::CurrentLevel;

pub struct LevelCompletionPlugin;

impl Plugin for LevelCompletionPlugin {
    fn build(&self, app: &mut App) {
        app.register_ldtk_entity::<CompletionMarkerBundle>("StartMarker")
            .register_ldtk_entity::<CompletionMarkerBundle>("EndMarker")
            .insert_resource(InProgressLevel(LevelIid::default()))
            .add_systems(Update, handle_start_end_markers);
    }
}

#[derive(Component)]
enum CompletionMarkerType {
    StartMarker,
    EndMarker,
}

#[derive(Bundle)]
struct CompletionMarkerBundle {
    marker_type: CompletionMarkerType,
    collider: Collider,
    sensor: Sensor,
    collision_groups: CollisionGroups,
}

#[derive(Resource)]
struct InProgressLevel(LevelIid);

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
            collider: Collider::cuboid(
                (entity_instance.width / 2) as f32,
                (entity_instance.height / 2) as f32,
            ),
            sensor: Sensor,
            collision_groups: CollisionGroups::new(
                GroupLabel::ALL,
                GroupLabel::PLAYER_COLLIDER | GroupLabel::PLAYER_SENSOR,
            ),
        }
    }
}

fn handle_start_end_markers(
    rapier_context: Query<&RapierContext>,
    q_player: Query<Entity, With<PlayerHurtMarker>>,
    q_completion_markers: Query<(Entity, &CompletionMarkerType), Without<PlayerHurtMarker>>,
    mut res_levels: ResMut<Levels>,
    res_current_level: Res<CurrentLevel>,
    mut res_in_progress_level: ResMut<InProgressLevel>,
) {
    let (Ok(rapier_context), Ok(player_entity), completion_markers) = (
        rapier_context.get_single(),
        q_player.get_single(),
        q_completion_markers.iter(),
    ) else {
        return;
    };
    for (marker_entity, marker_type) in completion_markers {
        let Some(true) = rapier_context.intersection_pair(marker_entity, player_entity) else {
            continue;
        };
        match marker_type {
            CompletionMarkerType::StartMarker => {
                res_in_progress_level.0 = res_current_level.level_iid.clone();
            }
            CompletionMarkerType::EndMarker => {
                let current = &res_current_level.level_iid;
                if res_in_progress_level.0 != *current {
                    return;
                }
                let mut unlock_next = false;
                for level in res_levels.0.iter_mut() {
                    if unlock_next {
                        level.locked = false;
                        break;
                    }
                    if level.level_iid == *current {
                        level.complete = true;
                        unlock_next = true;
                    }
                }
            }
        }
    }
}
