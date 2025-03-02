use bevy::prelude::*;
use bevy_ecs_ldtk::ldtk::Type;
use bevy_ecs_ldtk::{prelude::LdtkProject, LdtkProjectHandle};

use crate::level::get_ldtk_level_data;
use crate::level::start_flag::StartFlag;
use crate::player::PlayerMarker;
use crate::shared::{GameState, UiState};

pub struct LevelSelectPlugin;

const NUM_LEVELS: usize = 20;
const START_FLAG_IDENT: &'static str = "Start";

#[derive(Component)]
struct LevelSelectUiMarker;

#[derive(Component)]
struct LevelSelectButtonIndex(usize);

impl Plugin for LevelSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(UiState::LevelSelect), spawn_level_select)
            // .add_systems(OnEnter(UiState::LevelSelect), show_level_select)
            .add_systems(OnExit(UiState::LevelSelect), despawn_level_select)
            .add_systems(
                Update,
                handle_level_selection.run_if(in_state(UiState::LevelSelect)),
            );
    }
}

fn spawn_level_select(mut commands: Commands) {
    commands
        .spawn((
            LevelSelectUiMarker,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|parent| {
            for i in 1..=NUM_LEVELS {
                parent
                    .spawn((
                        Button,
                        Node {
                            width: Val::Percent(5.),
                            height: Val::Percent(5.),
                            ..default()
                        },
                        BorderColor(Color::WHITE),
                        LevelSelectButtonIndex(i),
                    ))
                    .with_child((Text::new(format!("{i}")),));
            }
        });
}

fn despawn_level_select(
    mut commands: Commands,
    mut level_select_ui_query: Query<Entity, With<LevelSelectUiMarker>>,
) {
    let Ok(entity) = level_select_ui_query.get_single_mut() else {
        panic!("Could not find level select ui!")
    };

    commands.entity(entity).despawn_recursive();
}

fn handle_level_selection(
    mut interaction_query: Query<
        (&Interaction, &LevelSelectButtonIndex),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_ui_state: ResMut<NextState<UiState>>,
    ldtk_assets: Res<Assets<LdtkProject>>,
    query_ldtk: Query<&LdtkProjectHandle>,
    mut query_player: Query<&mut Transform, (With<PlayerMarker>, Without<StartFlag>)>,
) {
    for (interaction, index) in &mut interaction_query {
        if *interaction != Interaction::Pressed {
            return;
        }

        let ldtk_levels = get_ldtk_level_data(ldtk_assets, query_ldtk);
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
                        player_transform.translation.y = -player_y as f32;
                        break 'loop_layers;
                    }
                }
            }
        }

        next_game_state.set(GameState::Playing);
        next_ui_state.set(UiState::None);
        break;
    }
}
