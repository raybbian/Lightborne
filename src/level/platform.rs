use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{player::PlayerMarker, shared::ResetLevel};

use super::LevelSystems;

/// Plugin for handling platforms
pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlatformToggleEvent>()
            //.add_systems(PreUpdate,(initialize_platforms,).in_set(LevelSystems::Processing).run_if(on_event::<ResetLevel>))
            .add_systems(Update, on_platform_changed.in_set(LevelSystems::Simulation).run_if(on_event::<ResetLevel>))
            .add_systems(FixedUpdate, move_platforms.in_set(LevelSystems::Simulation))
            .register_ldtk_entity::<MovingPlatformBundle>("MovingPlatform")
            //.add_systems(FixedUpdate, adjust_player.in_set(LevelSystems::Processing))
            .add_systems(FixedUpdate, reset_platforms.run_if(on_event::<ResetLevel>));
    }
}

#[derive(Clone, Copy, Debug)]
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


#[derive(Clone, Copy, Debug)]
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
    pub initial_state: PlatformState,
    pub speed: f32,
    pub width: i32,
    pub height: i32,
    pub curr_segment: IVec2,
    pub curr_segment_index: i32,
    pub curr_state: PlatformState,
    pub curr_direction: PlatformDirection,
    pub does_reverse: bool
}

impl From<&bevy_ecs_ldtk::EntityInstance> for MovingPlatform {
    fn from(entity_instance: &bevy_ecs_ldtk::EntityInstance) -> Self {
        let mut path = match &(*entity_instance.get_field_instance("path").unwrap()).value {
            FieldValue::Points(val) => val.clone().into_iter().flatten().collect::<Vec<IVec2>>(),
            _ => panic!("Unexpected data type!")
        };
        let speed = *entity_instance.get_float_field("speed").unwrap();
        let initial_state = PlatformState::from(entity_instance.get_enum_field("InitialState").unwrap());;
        let width = entity_instance.width;
        let height = entity_instance.height;
        let curr_segment = path[0].clone();
        let curr_segment_index = match path.len() {
            0 => 0,
            _ => 1
        };
        let initial_pos = IVec2::new(entity_instance.grid.x, entity_instance.grid.y);
        path.insert(0, initial_pos);
        let curr_state = initial_state;
        let curr_direction = PlatformDirection::Forward;
        let does_reverse = *entity_instance.get_bool_field("does_reverse").unwrap();

        MovingPlatform {
            path,
            initial_state,
            speed,
            width,
            height,
            curr_segment,
            curr_segment_index,
            curr_state,
            curr_direction,
            does_reverse
        }
    }
}

/// Bundle for platform physics
#[derive(Bundle)]
pub struct PlatformPhysicsBundle {
    pub rigid_body: RigidBody,
    pub collider: Collider,
    pub velocity: Velocity,
    pub friction: Friction
}

impl Default for PlatformPhysicsBundle {
    fn default() -> Self {
        Self {
            //rigid_body: RigidBody::KinematicVelocityBased,
            rigid_body: RigidBody::KinematicVelocityBased,
            collider: Collider::cuboid(8.0, 8.0), // shape of platform
            velocity: Velocity::zero(),
            friction: Friction {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min
            }
        }
    }
}

/// Bundle for moving platforms
#[derive(Default, Bundle, LdtkEntity)]
pub struct MovingPlatformBundle {
    #[from_entity_instance]
    pub platform: MovingPlatform,
    pub transform: Transform,
    #[grid_coords]
    pub grid_coords: GridCoords,
    pub global_transform: GlobalTransform,
    #[sprite("platform.png")]
    pub sprite: Sprite,
    pub physics: PlatformPhysicsBundle
}


/// Event that will toggle the movement of a platform
#[derive(Event)]
pub struct PlatformToggleEvent {
    pub state: bool
}

pub fn move_platforms(
    mut level_q: Query<(&mut MovingPlatform, &mut Transform, &mut RigidBody, &mut Velocity, Entity, &GlobalTransform), Without<PlayerMarker>>,
    mut player_q: Query<
        (
            Entity,
            & KinematicCharacterController,
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
    
    for (mut platform, mut transform, mut rigid_body, mut velocity, entity, global_transform) in level_q.iter_mut() {
        let path = platform.path.clone();
        let speed = platform.speed;
        let curr_segment = platform.curr_segment.clone();
        let curr_segment_index = platform.curr_segment_index;
        let curr_state = platform.curr_state;
        let current_position = Vec2::new((transform.translation.x / 8.0) - (platform.width as f32 / 2.0 / 8.0), 22.0 - (transform.translation.y / 8.0) + (platform.height as f32 / 2.0 / 8.0)); // NEED height of level and grid width
        let curr_direction = platform.curr_direction;
        
        let path_len = path.len() as i32;
        let mut previous_segment_index = (((curr_segment_index - 1) % path_len) + path_len) % path_len;

        let direction_vec = Vec2::new(curr_segment.x as f32 - current_position.x, -(curr_segment.y as f32 - current_position.y)).normalize();
        let direction_and_velocity = direction_vec * speed;

        let direction_vec_3d = Vec3::new(curr_segment.x as f32 - current_position.x, -(curr_segment.y as f32 - current_position.y), 0.0).normalize();
        transform.translation += direction_vec_3d * platform.speed * time.delta_secs();

        let relative_horizontal = global_transform.translation().x - player.4.translation();
        let horizontal_distance = relative_horizontal.x.abs() - 8.0; // player width is 8.0
        let relative_height = (player.4.translation().y - 9.5) - (global_transform.translation().y + (platform.height as f32 / 2.0)); // player height is 19
        let mut diff_sign = false;
        if (relative_horizontal.x > 0.0 && direction_vec.x < 0.0) || (relative_horizontal.x < 0.0 && direction_vec.x > 0.0) {
            diff_sign = true
        }
        if horizontal_distance <= platform.width as f32 / 2.0 && relative_height < 0.0 && relative_height > -(platform.height as f32 + 19.0) && diff_sign { // player height again
            player.3.translation.x += direction_vec.x * speed * time.delta_secs();
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
                player.3.translation += Vec3::new(direction_vec.x, direction_vec.y + 0.1, 0.0) * speed * time.delta_secs();
            }
        }

        let distance = current_position.distance(curr_segment.as_vec2());
        if distance <= 0.005 * platform.speed {
            platform.curr_segment = path[((platform.curr_segment_index + 1) % path_len) as usize];
            previous_segment_index = platform.curr_segment_index;
            platform.curr_segment_index = (previous_segment_index + 1) % path_len;
        }
    }
}

pub fn reset_platforms(
    mut platform_q: Query<(&mut MovingPlatform, &mut Transform)>
) {
    for (mut platform, mut transform) in platform_q.iter_mut() {
        transform.translation = Vec3::new((platform.path[0].x as f32 * 8.0) + (platform.width as f32 / 2.0), (22.0 * 8.0) - (platform.path[0].y as f32 * 8.0) + (platform.height as f32 / 2.0), 0.0);
        platform.curr_segment = platform.path[1]; // Bad, requires at least 1 point on path
    }
}

pub fn on_platform_changed() {
    println!("Not implemented yet!")
}

//pub fn reset_platforms() {
//
//}