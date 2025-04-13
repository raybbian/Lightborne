use std::f32::consts::PI;

use bevy::{
    math::ops::{cos, sin},
    prelude::*,
};
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{
    player::PlayerMarker,
    shared::{GroupLabel, ResetLevel},
};

use super::{CurrentLevel, LevelSystems};

const PLAYER_WIDTH: f32 = 16.0;
const PLAYER_HEIGHT: f32 = 19.0;
const BLOCK_WIDTH: f32 = 8.0;

/// [Plugin] for handling moving platformsd
pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            change_platform_state
                .in_set(LevelSystems::Simulation)
                .run_if(on_event::<ResetLevel>),
        )
        .add_systems(
            Update,
            change_platform_state
                .in_set(LevelSystems::Simulation)
                .run_if(on_event::<ChangePlatformStateEvent>),
        )
        .add_event::<ChangePlatformStateEvent>()
        .add_systems(FixedUpdate, move_platforms.in_set(LevelSystems::Simulation))
        .register_ldtk_entity::<MovingPlatformBundle>("MovingPlatform")
        .add_systems(FixedUpdate, reset_platforms.run_if(on_event::<ResetLevel>));
    }
}

/// Event for transitioning the state of all platforms with a specified id
#[derive(Event)]
pub struct ChangePlatformStateEvent {
    pub id: i32,
    pub new_state: PlatformState,
}

/// Enum for the state of a platform
#[derive(Default, Clone, PartialEq, Eq, Copy, Debug)]
pub enum PlatformState {
    #[default]
    Play,
    Pause,
    Stop,
}

// Convert Strings from LDtk Enums into true Enums
impl From<&String> for PlatformState {
    fn from(string: &String) -> Self {
        match string.as_str() {
            "Play" => PlatformState::Play,
            "Pause" => PlatformState::Pause,
            "Stop" => PlatformState::Stop,
            _ => PlatformState::Play,
        }
    }
}

/// Component to represent a moving platforms
#[derive(Default, Component)]
pub struct MovingPlatform {
    pub path: Vec<IVec2>, // Array of points that the platform will traverse
    pub path_curve_points: Vec<bool>, // Array of booleans determining circular motion of platform
    pub initial_state: PlatformState, // Initial state the platform spawns with
    pub speed: f32,       // Speed of the platform
    pub width: i32,       // Width of the platform in pixels
    pub height: i32,      // Height of the platform in pixels
    pub curr_segment: IVec2, // Current platform goal position
    pub previous_segment: IVec2, // Previous platform goal position (Used for circular motion)
    pub curr_segment_index: i32, // Index in "path" of the current platform goal
    pub curr_state: PlatformState, // The current state of the platform's motion
    pub does_repeat: bool, // Indicates if platform continues motion after reaching end of path
    pub can_reactivate: bool, // Indicates if platform can transition out of a Stop state if it has previously transitioned to a Stop state
    pub has_activated: bool, // Indicates if the platform has transitioned out of a Stop state (used by can_reactivate logic)
    pub id: i32,             // ID of the platform (used for event triggers)
    pub arc_time: f32, // Used to store current state of platform's motion during circular motion
    pub current_position: Vec2, // Stores the current position of the platform
}

impl MovingPlatform {
    fn get_next_direction_vec(&mut self, time: &Res<Time>) -> Vec2 {
        match self.path_curve_points[self.curr_segment_index as usize] {
            false => Vec2::new(
                self.curr_segment.x as f32 - self.current_position.x,
                -(self.curr_segment.y as f32 - self.current_position.y),
            )
            .normalize(),
            true => {
                let next_segment =
                    self.path[((self.curr_segment_index + 1) % self.path.len() as i32) as usize];

                let total_time =
                    (PI * 8.0 * (self.previous_segment.x as f32 - next_segment.x as f32).abs())
                        / (2.0 * self.speed);
                if self.curr_state == PlatformState::Play {
                    self.arc_time += time.delta_secs();
                }
                let curr_t = (self.arc_time / total_time) * PI / 2.0;

                let x_diff = next_segment.x - self.curr_segment.x;
                let y_diff = self.curr_segment.y - self.previous_segment.y;
                let other_y_diff = next_segment.y - self.curr_segment.y;
                let other_x_diff = self.curr_segment.x - self.previous_segment.x;
                match x_diff {
                    x if x < 0 => match y_diff {
                        x if x <= 0 => Vec2::new(-sin(curr_t), cos(curr_t)), // #5
                        _ => Vec2::new(-sin(curr_t), -cos(curr_t)),          // #2
                    },
                    x if x > 0 => match y_diff {
                        x if x <= 0 => Vec2::new(sin(curr_t), cos(curr_t)), // #6
                        _ => Vec2::new(sin(curr_t), -cos(curr_t)),          // #3
                    },
                    0 => match other_y_diff {
                        x if x >= 0 => match other_x_diff {
                            x if x >= 0 => Vec2::new(cos(curr_t), -sin(curr_t)), // #4
                            _ => Vec2::new(-cos(curr_t), -sin(curr_t)),          // 7
                        },
                        _ => match other_x_diff {
                            x if x >= 0 => Vec2::new(cos(curr_t), sin(curr_t)), // #1
                            _ => Vec2::new(-cos(curr_t), sin(curr_t)),          // #8
                        },
                    },
                    _ => unreachable!("Number somehow isn't in the range of all integers!"),
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn adjust_player(
        &self,
        player: &mut (
            Entity,
            Mut<'_, KinematicCharacterController>,
            &KinematicCharacterControllerOutput,
            Mut<'_, Transform>,
            &GlobalTransform,
        ),
        nearby_entities: (
            Option<Entity>,
            Option<Entity>,
            Option<Entity>,
            Option<Entity>,
        ),
        direction: Vec2,
        platform_entity: Entity,
        platform_global_transform: &GlobalTransform,
        ev_reset_level: &mut EventWriter<ResetLevel>,
        time: &Res<Time>,
    ) {
        let (
            entity_above_player,
            entity_below_player,
            entity_left_of_player,
            entity_right_of_player,
        ) = nearby_entities;
        let direction_and_velocity = direction * self.speed;

        let (_, _, player_controller_output, player_transform, player_global_transform) = player;

        // Crush player if platform is above player, moving down, and player is grounded
        if entity_above_player.is_some()
            && entity_above_player.unwrap().eq(&platform_entity)
            && player_controller_output.grounded
            && direction_and_velocity.y < 0.0
        {
            ev_reset_level.send(ResetLevel::Respawn);
            return;
        }

        // Move player with platform if player is standing on platform
        if entity_below_player.is_some() && entity_below_player.unwrap().eq(&platform_entity) {
            if self.curr_state == PlatformState::Play {
                // Crush player if platform moving player into ceiling
                if direction.y > 0.0 && entity_above_player.is_some() {
                    ev_reset_level.send(ResetLevel::Respawn);
                    return;
                }
                if (entity_left_of_player.is_none() || direction.x > 0.0)
                    && (entity_right_of_player.is_none() || direction.x < 0.0)
                {
                    player_transform.translation += Vec3::new(direction.x, direction.y + 0.1, 0.0)
                        * self.speed
                        * time.delta_secs();
                } else {
                    player_transform.translation +=
                        Vec3::new(0.0, direction.y + 0.1, 0.0) * self.speed * time.delta_secs();
                }
            } else {
                player_transform.translation += Vec3::new(0.0, 0.2, 0.0) * 1.0 * time.delta_secs();
            }
        }

        // Push player away from platform horizontally if platform is in Play state
        let relative_horizontal =
            platform_global_transform.translation().x - player_global_transform.translation();
        let horizontal_distance = relative_horizontal.x.abs() - (PLAYER_WIDTH / 2.0);
        let relative_height = (player_global_transform.translation().y - (PLAYER_HEIGHT / 2.0))
            - (platform_global_transform.translation().y + (self.height as f32 / 2.0));
        let mut diff_sign = false;
        if (relative_horizontal.x > 0.0 && direction.x < 0.0)
            || (relative_horizontal.x < 0.0 && direction.x > 0.0)
        {
            diff_sign = true
        }
        if horizontal_distance <= self.width as f32 / 2.0
            && relative_height < 0.0
            && relative_height > -(self.height as f32 + PLAYER_HEIGHT - 5.0)
            && diff_sign
        {
            // 5 pixel offset
            if self.curr_state == PlatformState::Play {
                if relative_horizontal.x < 0.0 {
                    if entity_right_of_player.is_some() {
                        ev_reset_level.send(ResetLevel::Respawn);
                        return;
                    }
                } else if entity_left_of_player.is_some() {
                    ev_reset_level.send(ResetLevel::Respawn);
                    return;
                }
                // Offset player if they are clipping into the platform
                let speed_adjustment = match horizontal_distance {
                    x if x > (self.width as f32 / 2.0) - 0.5 => 0.0,
                    _ => self.speed,
                };
                player_transform.translation.x +=
                    direction.x * (self.speed + speed_adjustment) * time.delta_secs();
            }
        }
    }
}

// Setting initial platform values and obtaining LDtk fields
impl From<&bevy_ecs_ldtk::EntityInstance> for MovingPlatform {
    fn from(entity_instance: &bevy_ecs_ldtk::EntityInstance) -> Self {
        let mut path = match &entity_instance.get_field_instance("path").unwrap().value {
            FieldValue::Points(val) => val.clone().into_iter().flatten().collect::<Vec<IVec2>>(),
            _ => panic!("Unexpected data type!"),
        };
        let mut path_curve_points = match &entity_instance
            .get_field_instance("path_curve_points")
            .unwrap()
            .value
        {
            FieldValue::Bools(val) => val.clone(),
            _ => panic!("Unexpected data type!"),
        };
        if path_curve_points.is_empty() {
            for _ in path.iter() {
                path_curve_points.insert(0, false);
            }
        }
        path_curve_points.insert(0, false);
        let speed = *entity_instance.get_float_field("speed").unwrap();
        let initial_state = PlatformState::from(entity_instance.get_enum_field("DefaultState").unwrap());
        let width = entity_instance.width;
        let height = entity_instance.height;
        let curr_segment = path[0];
        let curr_segment_index = match path.len() {
            0 => 0,
            _ => 1,
        };
        let initial_pos = IVec2::new(
            entity_instance.grid.x,
            entity_instance.grid.y + (height / (BLOCK_WIDTH as i32)) - 1,
        );
        path.insert(0, initial_pos);
        let previous_segment = initial_pos;
        let curr_state = initial_state;
        //let curr_direction = PlatformDirection::Forward;
        let does_reverse = *entity_instance.get_bool_field("does_reverse").unwrap();
        if does_reverse && path_curve_points[path_curve_points.len() - 1] {
            panic!("Last element of path_curve_points cannot be a curve if the platform reverses!");
        }
        let mut last_point = path_curve_points[0];
        for point in path_curve_points[1..].iter() {
            if last_point && last_point == *point {
                panic!("Elements in path_curve_points cannot be adjacent!");
            } else {
                last_point = *point;
            }
        }

        if does_reverse {
            let mut reversed_path = path.clone();
            reversed_path.reverse();
            path.append(&mut reversed_path);

            let mut reversed_path_curve_points = path_curve_points.clone();
            reversed_path_curve_points.reverse();
            path_curve_points.append(&mut reversed_path_curve_points);
        }

        let does_repeat = *entity_instance.get_bool_field("does_repeat").unwrap();
        let can_reactivate = *entity_instance.get_bool_field("can_reactivate").unwrap();
        let has_activated = false;
        let id = *entity_instance.get_int_field("event_id").unwrap();
        let arc_time = 0.0;
        let current_position = initial_pos.as_vec2();

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
            does_repeat,
            can_reactivate,
            has_activated,
            id,
            arc_time,
            current_position,
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
    pub collision_groups: CollisionGroups,
}

impl Default for PlatformPhysicsBundle {
    fn default() -> Self {
        Self {
            rigid_body: RigidBody::KinematicVelocityBased,
            collider: Collider::cuboid(12.0, 4.0), // shape of platform
            velocity: Velocity::zero(),
            friction: Friction {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            collision_groups: CollisionGroups::new(GroupLabel::PLATFORM, Group::ALL),
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
    #[sprite_sheet]
    pub sprite: Sprite,
    pub physics: PlatformPhysicsBundle,
}

/// [System] that moves platforms during each [Update] step
pub fn move_platforms(
    mut platform_q: Query<
        (
            &mut MovingPlatform,
            &mut Transform,
            Entity,
            &GlobalTransform,
        ),
        Without<PlayerMarker>,
    >,
    mut player_q: Query<
        (
            Entity,
            &mut KinematicCharacterController,
            &KinematicCharacterControllerOutput,
            &mut Transform,
            &GlobalTransform,
        ),
        With<PlayerMarker>,
    >,
    levels_q: Query<(Entity, &GlobalTransform, &LevelIid)>,
    parents: Query<&Parent>,
    levels: Query<&LevelIid>,
    rapier_context: ReadDefaultRapierContext,
    time: Res<Time>,
    mut ev_reset_level: EventWriter<ResetLevel>,
) {
    let Ok(mut player) = player_q.get_single_mut() else {
        return;
    };

    let (_, _, _, player_transform, _) = &player;

    //get entities that are near or intersecting player
    let entity_above_player = cast_player_ray_shape(
        &rapier_context,
        player_transform,
        0.0,
        5.5,
        12.0,
        0.0,
        Vec2::new(0.0, 1.0),
        GroupLabel::PLATFORM | GroupLabel::TERRAIN,
    );
    let entity_below_player = cast_player_ray_shape(
        &rapier_context,
        player_transform,
        0.0,
        -10.0,
        16.0,
        0.75,
        Vec2::new(0.0, -1.0),
        GroupLabel::PLATFORM,
    );
    let entity_left_of_player = cast_player_ray_shape(
        &rapier_context,
        player_transform,
        -8.0,
        -1.5,
        0.0,
        14.0,
        Vec2::new(-1.0, 0.0),
        GroupLabel::TERRAIN | GroupLabel::PLATFORM,
    ); // Check Y-Offset again
    let entity_right_of_player = cast_player_ray_shape(
        &rapier_context,
        player_transform,
        8.0,
        -1.5,
        0.0,
        14.0,
        Vec2::new(1.0, 0.0),
        GroupLabel::TERRAIN | GroupLabel::PLATFORM,
    );
    let nearby_entities = (
        entity_above_player,
        entity_below_player,
        entity_left_of_player,
        entity_right_of_player,
    );

    for (mut platform, mut transform, entity, global_transform) in platform_q.iter_mut() {
        // Calculate direction vector for platform motion (Depends on linear or circular motion)
        let direction_vec = platform.get_next_direction_vec(&time);
        //print!("X: {:?}, {:?} ", platform.curr_segment.x, platform.current_position.x);
        //println!("Y: {:?}, {:?}", platform.curr_segment.y, platform.current_position.y);
        //println!("{:?}", direction_vec);

        // Only move platform if it is in the Play state
        if platform.curr_state == PlatformState::Play {
            transform.translation += Vec3::new(direction_vec.x, direction_vec.y, 0.0)
                * platform.speed
                * time.delta_secs();
            
            let mut new_entity = entity;
            while let Ok(parent) = parents.get(new_entity) {
                new_entity = parent.get();
                if let Ok(_level_iid) = levels.get(new_entity) {
                    break;
                }
            }

            for (_entity, global_level_transform, id) in levels_q.iter() {
                if *id == *levels.get(new_entity).unwrap() {
                    let platform_translation =
                        global_transform.translation() - global_level_transform.translation();
                    platform.current_position = Vec2::new(
                        (platform_translation.x / BLOCK_WIDTH)
                            - (platform.width as f32 / 2.0 / BLOCK_WIDTH),
                        -(platform_translation.y / BLOCK_WIDTH)
                            - (platform.height as f32 / 2.0 / BLOCK_WIDTH)
                            + 23.0,
                    );
                }
            }
        }

        // Adjust the position of the player to prevent intersection and to move player with platform
        platform.adjust_player(
            &mut player,
            nearby_entities,
            direction_vec,
            entity,
            global_transform,
            &mut ev_reset_level,
            &time,
        );

        // Calculate distance to platform goal (Depends on linear or circular motion)
        let distance = match platform.path_curve_points[platform.curr_segment_index as usize] {
            false => platform
                .current_position
                .distance(platform.curr_segment.as_vec2()),
            true => {
                let next_segment = platform.path
                    [((platform.curr_segment_index + 1) % platform.path.len() as i32) as usize];
                platform.current_position.distance(next_segment.as_vec2())
            }
        };

        // Handles the transition of the platform's goal once it reaches it's current one (Skips a goal if circular motion)
        if distance <= 0.005 * platform.speed {
            platform.previous_segment = platform.curr_segment;
            if platform.path_curve_points[platform.curr_segment_index as usize] {
                platform.arc_time = 0.0;
                platform.curr_segment_index =
                    (platform.curr_segment_index + 1) % platform.path.len() as i32;
                platform.curr_segment = platform.path[platform.curr_segment_index as usize];
                platform.previous_segment = platform.curr_segment;
            }
            if platform.curr_segment_index == platform.path.len() as i32 - 1 {
                platform.curr_segment_index = 0;
                platform.curr_segment = platform.path[platform.curr_segment_index as usize];
                if !platform.does_repeat {
                    platform.has_activated = true;
                    platform.curr_state = PlatformState::Stop;
                }
            } else {
                platform.curr_segment_index += 1;
                platform.curr_segment = platform.path[platform.curr_segment_index as usize];
            }
        }
    }
}

/// [System] that resets the state of all platforms
pub fn reset_platforms(mut platform_q: Query<(&mut MovingPlatform, &mut Transform)>) {
    for (mut platform, mut transform) in platform_q.iter_mut() {
        transform.translation = Vec3::new(
            (platform.path[0].x as f32 * BLOCK_WIDTH) + (platform.width as f32 / 2.0),
            (22.0 * BLOCK_WIDTH) - (platform.path[0].y as f32 * BLOCK_WIDTH)
                + (platform.height as f32 / 2.0),
            0.0,
        );
        platform.curr_segment = match platform.path.len() {
            1 => platform.path[0],
            _ => platform.path[1],
        };
        platform.previous_segment = platform.path[0];
        platform.curr_segment_index = 1;
        platform.curr_state = platform.initial_state;
        platform.arc_time = 0.0;
        platform.current_position = Vec2::new(platform.path[0].x as f32, platform.path[0].y as f32);
    }
}

/// function that casts a ray shape relative to the player
#[allow(clippy::too_many_arguments)]
fn cast_player_ray_shape(
    rapier_context: &ReadDefaultRapierContext,
    player_transform: &Transform,
    x_offset: f32,
    y_offset: f32,
    width: f32,
    height: f32,
    dir: Vec2,
    filter: Group,
) -> Option<Entity> {
    let mut entity_near_player: Option<Entity>;
    entity_near_player = None;
    if let Some((found_entity, _)) = rapier_context.cast_shape(
        // Note: Query Filter is needed!
        Vec2::new(
            player_transform.translation.x + x_offset,
            player_transform.translation.y + y_offset,
        ),
        0.0,
        dir,
        &Collider::cuboid(width / 2.0, height / 2.0),
        ShapeCastOptions {
            max_time_of_impact: 0.0,
            target_distance: 0.0,
            stop_at_penetration: true,
            compute_impact_geometry_on_penetration: false,
        },
        QueryFilter::new().groups(CollisionGroups::new(GroupLabel::ALL, filter)),
    ) {
        entity_near_player = Some(found_entity);
    }
    entity_near_player
}

/// [System] that checks for [ChangePlatformStateEvent] [Event] during each [Update] step and updates the platform's state accordingly
pub fn change_platform_state(
    mut event_reader: EventReader<ChangePlatformStateEvent>,
    mut platform_q: Query<(Entity, &mut MovingPlatform)>,
    current_level: Res<CurrentLevel>,
    parents: Query<&Parent>,
    levels: Query<&LevelIid>,
) {
    for event in event_reader.read() {
        match event.new_state {
            PlatformState::Play => {
                //println!("Platform found!");
                for (entity, mut platform) in platform_q.iter_mut() {
                    //println!("There is a platform");
                    let mut new_entity = entity;
                    while let Ok(parent) = parents.get(new_entity) {
                        new_entity = parent.get();
                        if let Ok(_level_iid) = levels.get(new_entity) {
                            break;
                        }
                    }
                    //println!("{:?}", levels.get(new_entity));
                    //println!("{:?}", current_level.level_iid);
                    //println!("{:?} {:?}", platform.id, event.id);
                    //println!("{:?}", platform.path);
                    //println!("{:?}", platform.curr_segment);
                    if platform.id == event.id
                        && current_level.level_iid == *levels.get(new_entity).unwrap()
                    {
                        //println!("Platform in level");
                        platform.curr_state = match platform.curr_state {
                            PlatformState::Play => PlatformState::Play,
                            PlatformState::Pause => PlatformState::Play,
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
            }
            PlatformState::Pause => {
                for (entity, mut platform) in platform_q.iter_mut() {
                    let mut new_entity = entity;
                    while let Ok(parent) = parents.get(new_entity) {
                        new_entity = parent.get();
                        if let Ok(_level_iid) = levels.get(new_entity) {
                            break;
                        }
                    }
                    if platform.id == event.id
                        && current_level.level_iid == *levels.get(new_entity).unwrap()
                    {
                        platform.curr_state = match platform.curr_state {
                            PlatformState::Play => PlatformState::Pause,
                            PlatformState::Pause => PlatformState::Pause,
                            PlatformState::Stop => PlatformState::Stop,
                        };
                    }
                }
            }
            PlatformState::Stop => {
                for (entity, mut platform) in platform_q.iter_mut() {
                    let mut new_entity = entity;
                    while let Ok(parent) = parents.get(new_entity) {
                        new_entity = parent.get();
                        if let Ok(_level_iid) = levels.get(new_entity) {
                            break;
                        }
                    }
                    if platform.id == event.id
                        && current_level.level_iid == *levels.get(entity).unwrap()
                    {
                        platform.curr_state = match platform.curr_state {
                            PlatformState::Play => {
                                platform.has_activated = true;
                                PlatformState::Stop
                            }
                            PlatformState::Pause => PlatformState::Stop,
                            PlatformState::Stop => PlatformState::Stop,
                        };
                    }
                }
            }
        }
    }
}
