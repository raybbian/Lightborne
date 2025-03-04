use bevy::prelude::*;
use bevy_ecs_ldtk::ldtk::Type;
use bevy_ecs_ldtk::LevelIid;
use bevy_ecs_ldtk::{prelude::LdtkProject, LdtkProjectHandle};

use crate::camera::handle_move_camera;
use crate::level::start_flag::StartFlag;
use crate::level::{get_ldtk_level_data, CurrentLevel};
use crate::player::PlayerMarker;
use crate::shared::{GameState, UiState, LYRA_RESPAWN_EPSILON};

pub struct LevelSelectPlugin;

const START_FLAG_IDENT: &str = "Start";

#[derive(Component)]
struct LevelSelectUiMarker;

#[derive(Component)]
pub struct LevelSelectButtonIndex(usize);

impl Plugin for LevelSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                spawn_level_select.run_if(in_state(UiState::LevelSelect)),
                despawn_level_select
                    .after(handle_move_camera)
                    .run_if(not(in_state(UiState::LevelSelect))),
                handle_level_selection.run_if(in_state(UiState::LevelSelect)),
            ),
        );
    }
}

fn spawn_level_select(
    mut commands: Commands,
    ldtk_assets: Res<Assets<LdtkProject>>,
    query_ldtk: Query<&LdtkProjectHandle>,
    level_select_ui_query: Query<Entity, With<LevelSelectUiMarker>>,
) {
    if level_select_ui_query.get_single().is_ok() {
        return;
    }
    let Ok(levels) = get_ldtk_level_data(ldtk_assets, query_ldtk) else {
        return;
    };
    let num_levels = levels.len();
    commands
        .spawn((
            LevelSelectUiMarker,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::SpaceBetween,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|parent| {
            parent.spawn(Text::new("Level Select"));
            parent
                .spawn(Node {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    ..default()
                })
                .with_children(|parent| {
                    for i in 0..num_levels {
                        parent
                            .spawn((
                                Button,
                                Node {
                                    width: Val::Percent(4.),
                                    height: Val::Percent(4.),
                                    border: UiRect::all(Val::Percent(0.2)),
                                    margin: UiRect::all(Val::Percent(0.5)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BorderColor(Color::WHITE),
                                LevelSelectButtonIndex(i),
                            ))
                            .with_child((Text::new(format!("{i}")),));
                    }
                });
        });
}

fn despawn_level_select(
    mut commands: Commands,
    mut level_select_ui_query: Query<Entity, With<LevelSelectUiMarker>>,
) {
    let Ok(entity) = level_select_ui_query.get_single_mut() else {
        return;
    };

    commands.entity(entity).despawn_recursive();
}

#[allow(clippy::type_complexity)]
pub fn handle_level_selection(
    mut interaction_query: Query<
        (&Interaction, &LevelSelectButtonIndex),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_ui_state: ResMut<NextState<UiState>>,
    ldtk_assets: Res<Assets<LdtkProject>>,
    query_ldtk: Query<&LdtkProjectHandle>,
    mut query_player: Query<&mut Transform, (With<PlayerMarker>, Without<StartFlag>)>,
    mut current_level: ResMut<CurrentLevel>,
) {
    // We expect there to be only one interaction
    if let Some((interaction, index)) = (&mut interaction_query).into_iter().next() {
        if *interaction != Interaction::Pressed {
            return;
        }

        let Ok(ldtk_levels) = get_ldtk_level_data(ldtk_assets, query_ldtk) else {
            return;
        };
        if index.0 >= ldtk_levels.len() {
            panic!("Selected level index is out of bounds!")
        }
        let Some(layers) = ldtk_levels[index.0].layer_instances.as_ref() else {
            panic!("Layers not found! (This is probably because you are using the \"Separate level files\" option.)")
        };
        'loop_layers: for layer in layers {
            if layer.layer_instance_type == Type::Entities {
                for entity in &layer.entity_instances {
                    if entity.identifier == START_FLAG_IDENT {
                        let (Some(player_x), Some(player_y)) = (entity.world_x, entity.world_y)
                        else {
                            panic!("Start flag entity has no coordinates! (This is probably because your LDTK world is not in free layout mode.)");
                        };
                        let Ok(mut player_transform) = query_player.get_single_mut() else {
                            panic!("Could not find player!");
                        };
                        player_transform.translation.x = player_x as f32;
                        player_transform.translation.y = -player_y as f32 + LYRA_RESPAWN_EPSILON;
                        break 'loop_layers;
                    }
                }
            }
        }

        next_game_state.set(GameState::Playing);
        next_ui_state.set(UiState::None);
        // Set the current level_iid to an empty string so we don't trigger the camera transition (skull emoji)
        current_level.level_iid = LevelIid::new("");
    }
}
