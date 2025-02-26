use std::f32::consts::PI;

use bevy::{ math::ops::{cos, sin}, prelude::*};
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{player::PlayerMarker, shared::ResetLevel};

use super::LevelSystems;

/// Plugin for handling platforms
pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, change_platform_state.in_set(LevelSystems::Simulation).run_if(on_event::<ResetLevel>))
            .add_systems(Update, change_platform_state.in_set(LevelSystems::Simulation).run_if(on_event::<ChangePlatformState>))
            .add_event::<ChangePlatformState>()
            .add_systems(FixedUpdate, move_platforms.in_set(LevelSystems::Simulation))
            .register_ldtk_entity::<MovingPlatformBundle>("MovingPlatform")
            .add_systems(FixedUpdate, reset_platforms.run_if(on_event::<ResetLevel>));
    }
}

#[derive(Event)]
pub enum ChangePlatformState {
    Play {
        id: i32
    },
    Pause {
        id: i32
    },
    Stop {
        id: i32
    }
}

#[derive(Clone, PartialEq, Eq, Copy, Debug)]
pub enum PlatformState {
    Play,
    Pause,
    Stop
}

impl Default for PlatformState {
    fn default() -> Self {
        PlatformState::Play
    }
}

impl From<&String> for PlatformState {
    fn from(string: &String) -> Self {
        match string.as_str() {
            "Play" => PlatformState::Play,
            "Pause" => PlatformState::Pause,
            "Stop" => PlatformState::Stop,
            _ => PlatformState::Play
        }
    }
}


#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum PlatformDirection {
    Forward,
    Backward
}

impl Default for PlatformDirection {
    fn default() -> Self {
        PlatformDirection::Forward
    }
}

/// Component for moving platforms
#[derive(Default, Component)]
pub struct MovingPlatform {
    pub path: Vec<IVec2>,
    pub path_curve_points: Vec<bool>,
    pub initial_state: PlatformState,
    pub speed: f32,
    pub width: i32,
    pub height: i32,
    pub curr_segment: IVec2,
    pub previous_segment: IVec2,
    pub curr_segment_index: i32,
    pub curr_state: PlatformState,
    pub curr_direction: PlatformDirection,
    pub does_reverse: bool,
    pub does_repeat: bool,
    pub can_reactivate: bool,
    pub has_activated: bool,
    pub id: i32,
    pub arc_time: f32
}

impl From<&bevy_ecs_ldtk::EntityInstance> for MovingPlatform {
    fn from(entity_instance: &bevy_ecs_ldtk::EntityInstance) -> Self {
        let mut path = match &(*entity_instance.get_field_instance("path").unwrap()).value {
            FieldValue::Points(val) => val.clone().into_iter().flatten().collect::<Vec<IVec2>>(),
            _ => panic!("Unexpected data type!")
        };
        let mut path_curve_points = match &(*entity_instance.get_field_instance("path_curve_points").unwrap()).value {
            FieldValue::Bools(val) => val.clone(),
            _ => panic!("Unexpected data type!")
        };
        if path_curve_points.len() == 0 {
            for _ in path.iter() {
                path_curve_points.insert(0, false);
            }
        }
        path_curve_points.insert(0, false);
        let speed = *entity_instance.get_float_field("speed").unwrap();
        let initial_state = PlatformState::from(entity_instance.get_enum_field("InitialState").unwrap());
        let width = entity_instance.width;
        let height = entity_instance.height;
        let curr_segment = path[0].clone();
        let curr_segment_index = match path.len() {
            0 => 0,
            _ => 1
        };
        let initial_pos = IVec2::new(entity_instance.grid.x, entity_instance.grid.y);
        path.insert(0, initial_pos);
        let previous_segment = initial_pos;
        let curr_state = initial_state;
        let curr_direction = PlatformDirection::Forward;
        let does_reverse = *entity_instance.get_bool_field("does_reverse").unwrap();
        if does_reverse && path_curve_points[path_curve_points.len() - 1] == true {
            panic!("Last element of path_curve_points cannot be a curve if the platform reverses!");
        }
        let does_repeat = *entity_instance.get_bool_field("does_repeat").unwrap();
        let can_reactivate = *entity_instance.get_bool_field("can_reactivate").unwrap();
        let has_activated = false;
        let id = *entity_instance.get_int_field("event_id").unwrap();
        let arc_time = 0.0;

        MovingPlatform {
            path,
            path_curve_points,
            initial_state,
            speed,
            width,
            height,
            curr_segment,
            previous_segment,
            curr_segment_index,
            curr_state,
            curr_direction,
            does_reverse,
            does_repeat,
            can_reactivate,
            has_activated,
            id,
            arc_time
        }
    }
}

/// Bundle for platform physics
#[derive(Bundle)]
pub struct PlatformPhysicsBundle {
    pub rigid_body: RigidBody,
    pub collider: Collider,
    pub velocity: Velocity,
    pub friction: Friction,
    pub transform: Transform
}

impl Default for PlatformPhysicsBundle {
    fn default() -> Self {
        Self {
            rigid_body: RigidBody::KinematicVelocityBased,
            collider: Collider::cuboid(4.0, 4.0), // shape of platform
            velocity: Velocity::zero(),
            friction: Friction {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min
            },
            transform: Default::default()
        }
    }
}

/// Bundle for moving platforms
#[derive(Default, Bundle, LdtkEntity)]
pub struct MovingPlatformBundle {
    #[from_entity_instance]
    pub platform: MovingPlatform,
    #[grid_coords]
    pub grid_coords: GridCoords,
    pub global_transform: GlobalTransform,
    #[sprite("platform.png")]
    pub sprite: Sprite,
    pub physics: PlatformPhysicsBundle
}

pub fn move_platforms(
    mut level_q: Query<(&mut MovingPlatform, &mut Transform, Entity, &GlobalTransform), Without<PlayerMarker>>,
    mut player_q: Query<
        (
            Entity,
            &KinematicCharacterController,
            & KinematicCharacterControllerOutput,
            &mut Transform,
            & GlobalTransform
        ),
        With<PlayerMarker>
    >,
    rapier_context: ReadDefaultRapierContext,
    time: Res<Time>,
    mut ev_reset_level: EventWriter<ResetLevel>
) {
    let Ok(mut player) = player_q.get_single_mut() else {
        return
    };
    let mut entity_below_player: Option<Entity>;
    entity_below_player = None;
    let mut entity_above_player: Option<Entity>;
    entity_above_player = None;
    if let Some((found_entity, _)) = rapier_context.cast_shape( // Note: Query Filter is needed!
        Vec2::new(player.3.translation.x, player.3.translation.y - 10.0),
        0.0,
        Vec2::new(0.0, -1.0),
        &Collider::cuboid(8.0, 0.375),
        ShapeCastOptions {
            max_time_of_impact: 0.0,
            target_distance: 0.0,
            stop_at_penetration: true,
            compute_impact_geometry_on_penetration: false
        },
        QueryFilter::default()
    ) {
        entity_below_player = Some(found_entity);
    }

    if let Some((found_entity, _)) = rapier_context.cast_shape( // Note: Query Filter is needed!
        Vec2::new(player.3.translation.x, player.3.translation.y + 10.0),
        0.0,
        Vec2::new(0.0, -1.0),
        &Collider::cuboid(8.0, 0.375),
        ShapeCastOptions {
            max_time_of_impact: 0.0,
            target_distance: 0.0,
            stop_at_penetration: true,
            compute_impact_geometry_on_penetration: false
        },
        QueryFilter::default()
    ) {
        entity_above_player = Some(found_entity);
    }
    for (mut platform, mut transform, entity, global_transform) in level_q.iter_mut() {
        let curr_state = platform.curr_state;
        let path = platform.path.clone();
        let speed = platform.speed;
        let curr_segment = platform.curr_segment.clone();
        let previous_segment = platform.previous_segment;
        let mut curr_segment_index = platform.curr_segment_index;
        let current_position = Vec2::new((transform.translation.x / 8.0) - (platform.width as f32 / 2.0 / 8.0), 22.0 - (transform.translation.y / 8.0) + (platform.height as f32 / 2.0 / 8.0)); // NEED height of level and grid width
        let curr_direction = platform.curr_direction;
        let does_reverse = platform.does_reverse;
        let does_repeat = platform.does_repeat;
        
        let path_len = path.len() as i32;

        let direction_vec = match platform.path_curve_points[curr_segment_index as usize] {
            false => Vec2::new(curr_segment.x as f32 - current_position.x, -(curr_segment.y as f32 - current_position.y)).normalize(),
            true => {
                let next_segment = match platform.curr_direction {
                    PlatformDirection::Forward => path[((curr_segment_index + 1) % path_len) as usize],
                    PlatformDirection::Backward => path[(curr_segment_index - 1) as usize]
                };
                
                let total_time = (PI * 8.0 * (previous_segment.x as f32 - next_segment.x as f32).abs()) / (2.0 * speed);
                if curr_state == PlatformState::Play {
                    platform.arc_time = platform.arc_time + time.delta_secs();
                }
                let curr_t = (platform.arc_time / total_time) * PI / 2.0;

                let x_diff = next_segment.x - curr_segment.x;
                let y_diff = curr_segment.y - previous_segment.y;
                let other_y_diff = next_segment.y - curr_segment.y;
                let other_x_diff = curr_segment.x - previous_segment.x;
                match x_diff {
                    x if x < 0 => match y_diff {
                        x if x <= 0 => Vec2::new(-sin(curr_t), cos(curr_t)), // #5
                        _ => Vec2::new(-sin(curr_t), -cos(curr_t)) // #2
                    },
                    x if x > 0 => match  y_diff {
                        x if x <= 0 => Vec2::new(sin(curr_t), cos(curr_t)), // #6
                        _ => Vec2::new(sin(curr_t), -cos(curr_t)) // #3
                    },
                    0 => match other_y_diff {
                        x if x >= 0 => match other_x_diff {
                            x if x >= 0 => Vec2::new(cos(curr_t), -sin(curr_t)), // #4
                            _ => Vec2::new(-cos(curr_t), -sin(curr_t)) // 7
                        },
                        _ => match other_x_diff {
                            x if x >= 0 => Vec2::new(cos(curr_t), sin(curr_t)), // #1
                            _ => Vec2::new(-cos(curr_t), sin(curr_t)) // #8
                        }
                    },
                    _ => Vec2::new(0.0, 0.0) // unreachable
                }
            }
        };

        let direction_and_velocity = direction_vec * speed;

        if curr_state == PlatformState::Play {
            transform.translation += Vec3::new(direction_vec.x, direction_vec.y, 0.0) * platform.speed * time.delta_secs();
        }

        let relative_horizontal = global_transform.translation().x - player.4.translation();
        let horizontal_distance = relative_horizontal.x.abs() - 8.0; // player width is 8.0
        let relative_height = (player.4.translation().y - 9.5) - (global_transform.translation().y + (platform.height as f32 / 2.0)); // player height is 19
        let mut diff_sign = false;
        if (relative_horizontal.x > 0.0 && direction_vec.x < 0.0) || (relative_horizontal.x < 0.0 && direction_vec.x > 0.0) {
            diff_sign = true
        }
        if horizontal_distance <= platform.width as f32 / 2.0 && relative_height < 0.0 && relative_height > -(platform.height as f32 + 19.0) && diff_sign { // player height again
            if curr_state == PlatformState::Play {
                player.3.translation.x += direction_vec.x * speed * time.delta_secs();
            }
        }
        
        if !entity_above_player.is_none() {
            if entity_above_player.unwrap().eq(&entity) {
                if player.2.grounded && direction_and_velocity.y < 0.0 {
                    ev_reset_level.send(ResetLevel::Respawn);
                }
            }
        }
        

        if !entity_below_player.is_none() {
            if entity_below_player.unwrap().eq(&entity) {
                if curr_state == PlatformState::Play {
                    player.3.translation += Vec3::new(direction_vec.x, direction_vec.y + 0.1, 0.0) * speed * time.delta_secs();
                } else {
                    player.3.translation += Vec3::new(0.0, 0.2, 0.0) * 1.0 * time.delta_secs();
                }
            }
        }

        // Logic for handling transition of platform goal when each segment is reached
        let distance = match platform.path_curve_points[curr_segment_index as usize] {
            false => current_position.distance(curr_segment.as_vec2()),
            true => {
                let next_segment = match platform.curr_direction {
                    PlatformDirection::Forward => path[((curr_segment_index + 1) % path_len) as usize],
                    PlatformDirection::Backward => path[(curr_segment_index - 1) as usize]
                };
                current_position.distance(next_segment.as_vec2())
            }
        };
        if distance <= 0.005 * platform.speed {
            platform.previous_segment = curr_segment;
            if platform.path_curve_points[curr_segment_index as usize] == true {
                platform.arc_time = 0.0;
                platform.curr_segment = match platform.curr_direction {
                    PlatformDirection::Forward => {
                        platform.curr_segment_index = (platform.curr_segment_index + 1) % path_len;
                        curr_segment_index = platform.curr_segment_index;
                        path[curr_segment_index as usize]
                    },
                    PlatformDirection::Backward => {
                        platform.curr_segment_index -= 1;
                        curr_segment_index = platform.curr_segment_index;
                        path[curr_segment_index as usize]
                    }
                };
                platform.previous_segment = platform.curr_segment;
            }
            if curr_direction == PlatformDirection::Forward {
                if curr_segment_index == path_len - 1 {
                    if does_reverse {
                        platform.curr_direction = PlatformDirection::Backward;
                        platform.curr_segment_index -= 1;
                        platform.curr_segment = path[platform.curr_segment_index as usize];
                    } else {
                        platform.curr_segment_index = 0;
                        platform.curr_segment = path[platform.curr_segment_index as usize];
                    }
                    if !does_repeat {
                        platform.has_activated = true;
                        platform.curr_state = PlatformState::Stop;
                    }
                } else {
                    platform.curr_segment_index += 1;
                    platform.curr_segment = path[platform.curr_segment_index as usize];
                }
            } else {
                if curr_segment_index == 0 {
                    if does_reverse {
                        platform.curr_direction = PlatformDirection::Forward;
                        platform.curr_segment_index += 1;
                        platform.curr_segment = path[platform.curr_segment_index as usize];
                    } else {
                        platform.curr_segment_index = path_len - 1;
                        platform.curr_segment = path[platform.curr_segment_index as usize];
                    }
                    if !does_repeat {
                        platform.has_activated = true;
                        platform.curr_state = PlatformState::Stop;
                    }
                } else {
                    platform.curr_segment_index -= 1;
                    platform.curr_segment = path[platform.curr_segment_index as usize];
                }
            }
        }
    }
}

pub fn reset_platforms(
    mut platform_q: Query<(&mut MovingPlatform, &mut Transform)>
) {
    for (mut platform, mut transform) in platform_q.iter_mut() {
        transform.translation = Vec3::new((platform.path[0].x as f32 * 8.0) + (platform.width as f32 / 2.0), (22.0 * 8.0) - (platform.path[0].y as f32 * 8.0) + (platform.height as f32 / 2.0), 0.0);
        platform.curr_segment = match platform.path.len() {
            1 => platform.path[0],
            _ => platform.path[1]
        };
        platform.previous_segment = platform.path[0];
        platform.curr_segment_index = 1;
        platform.curr_state = platform.initial_state;
        platform.arc_time = 0.0;
    }
}

pub fn change_platform_state(
    mut event_reader: EventReader<ChangePlatformState>,
    mut platform_q: Query<&mut MovingPlatform>
) {
    for event in event_reader.read() {
        match event {
            ChangePlatformState::Play {id}=>  {
                for mut platform in platform_q.iter_mut() {
                    if platform.id == *id {
                        platform.curr_state = match platform.curr_state {
                            PlatformState::Play => {
                                PlatformState::Play
                            },
                            PlatformState::Pause => {
                                PlatformState::Play
                            },
                            PlatformState::Stop => {
                                if !platform.can_reactivate && platform.has_activated {
                                    PlatformState::Stop
                                } else {
                                    PlatformState::Play
                                }
                            }
                        };
                    }
                }
            },
            ChangePlatformState::Pause {id}=> {
                for mut platform in platform_q.iter_mut() {
                    if platform.id == *id {
                        platform.curr_state = match platform.curr_state {
                            PlatformState::Play => {
                                PlatformState::Pause
                            },
                            PlatformState::Pause => {
                                PlatformState::Pause
                            },
                            PlatformState::Stop => {
                                PlatformState::Stop
                            }
                        };
                    }
                }

            },
            ChangePlatformState::Stop {id}=> {
                for mut platform in platform_q.iter_mut() {
                    if platform.id == *id {
                        platform.curr_state = match platform.curr_state {
                            PlatformState::Play => {
                                platform.has_activated = true;
                                PlatformState::Stop
                            },
                            PlatformState::Pause => {
                                PlatformState::Stop
                            },
                            PlatformState::Stop => {
                                PlatformState::Stop
                            }
                        };
                    }
                }
                
            },
        }
    }
}