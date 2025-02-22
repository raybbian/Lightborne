use std::{default, mem::discriminant};

use bevy::{prelude::*, state::commands, text::cosmic_text::ttf_parser::loca, utils::hashbrown::hash_map::IterMut};
use bevy_ecs_ldtk::{ldtk::{ldtk_fields, FieldInstance}, prelude::*};
use bevy_rapier2d::prelude::*;
use bevy::log::LogPlugin;
use bevy::math;

use crate::{player::{PlayerBundle, PlayerMarker}, shared::{GameState, GroupLabel, ResetLevel}};

use super::{entity, LevelSystems};
use super::super::player::movement::PlayerMovement;


/// Plugin for handling platforms
pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlatformToggleEvent>()
            //.add_systems(PreUpdate,(initialize_platforms,).in_set(LevelSystems::Processing).run_if(on_event::<ResetLevel>))
            .add_systems(Update, on_platform_changed.in_set(LevelSystems::Simulation).run_if(on_event::<ResetLevel>))
            .add_systems(FixedUpdate, move_platforms.in_set(LevelSystems::Processing))
            .register_ldtk_entity::<MovingPlatformBundle>("MovingPlatform")
            .add_systems(PreUpdate, initialize_platforms.in_set(LevelSystems::Simulation))
            .add_systems(FixedUpdate, adjust_player.in_set(LevelSystems::Processing))
            .add_systems(FixedUpdate, reset_platforms.run_if(on_event::<ResetLevel>));
    }
}

/// Component for moving platforms
#[derive(Default, Component)]
pub struct MovingPlatform {
    pub path: Vec<IVec2>,
    pub default_state: bool,
    pub speed: f32,
    pub width: i32,
    pub height: i32,
    pub is_init: Option<bool>,
    pub curr_segment: Option<IVec2>,
    pub curr_segment_index: Option<i32>,
    pub initial_pos: Option<Vec2>,
    pub curr_state: Option<i32>,
    pub curr_direction: Option<bool>,
    pub curr_velocity: Option<Vec2>
}

impl From<&bevy_ecs_ldtk::EntityInstance> for MovingPlatform {
    fn from(entity_instance: &bevy_ecs_ldtk::EntityInstance) -> Self {
        let path = match &(*entity_instance.get_field_instance("path").unwrap()).value {
            FieldValue::Points(val) => val.clone().into_iter().flatten().collect(),
            _ => panic!("Unexpected data type!")
        };
        let speed = *entity_instance.get_float_field("speed").unwrap();
        let default_state = *entity_instance.get_bool_field("default_state").unwrap();
        let width = entity_instance.width;
        let height = entity_instance.height;

        MovingPlatform {
            path,
            default_state,
            speed,
            height,
            width,
            ..Default::default()
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
            rigid_body: RigidBody::KinematicPositionBased,
            collider: Collider::cuboid(8.0, 8.0), // shape of platform
            velocity: Velocity::zero(),
            friction: Friction {
                coefficient: 10.0,
                combine_rule: CoefficientCombineRule::Multiply
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


/**
 * Supposed to set collision groups
 */
pub fn initialize_platforms(
    mut commands: Commands,
    mut level_q: Query<(&mut MovingPlatform, &mut Transform, &GlobalTransform, &mut RigidBody, &mut Collider, &GridCoords), Added<MovingPlatform>>,
    player_q: Query<&GlobalTransform, With<PlayerMarker>>,
    asset_server: Res<AssetServer>
) {
    for (mut platform, mut local_transform, global_transform, mut rigid_body, mut collider, grid_coords) in level_q.iter_mut() {
        // Initialization:

        println!("Added platform!");
        if platform.curr_segment.is_none() {
            println!("Path is {:?} ", platform.path);
            println!("Default State is {:?} ", platform.default_state);
            platform.curr_segment = Some(platform.path[0]);
        }

        if platform.curr_segment_index.is_none() {
            platform.curr_segment_index = Some(1);
        }

        if platform.initial_pos.is_none() {
            //let initial_position = Vec2::new(local_transform.translation.x.round(), local_transform.translation.y.round());
            let initial_position = Vec2::new(grid_coords.x as f32, grid_coords.y as f32);
            //platform.initial_pos = Some(initial_position.clone());
            platform.initial_pos = Some(initial_position);
            platform.path.insert(0, IVec2::new(initial_position.x as i32, 22 - (initial_position.y as i32))); //NEED height of level
            //for mut vector in &mut platform.path {
                //*vector = *vector * 2;
            //}
            print!("path: {:?} ", platform.path);
        }

        if platform.curr_direction.is_none() {
            platform.curr_direction = Some(true);
        }

        if platform.curr_state.is_none() {
            platform.curr_state = match platform.default_state {
                true => Some(1),
                false => Some(0)
            }
        }

        //gravity.0 = 0.0;
        //*locked_axes = LockedAxes::ROTATION_LOCKED;
        *rigid_body = RigidBody::KinematicPositionBased;
        *collider = Collider::cuboid(platform.width as f32, platform.height as f32); // shape of platform
        platform.is_init = Some(true);
    }
}

pub fn move_platforms(
    mut level_q: Query<(&mut MovingPlatform, &mut Transform, &mut RigidBody, &mut Velocity)>,
    time: Res<Time>
) {
    for (mut platform, mut transform, mut rigid_body, mut velocity) in level_q.iter_mut() {
        if platform.is_init.is_none() {
            continue;
        }
        let path = platform.path.clone();
        //print!("Path is {:?} ", platform.path);
        let speed = platform.speed;
        let curr_segment = platform.curr_segment.clone();
        let curr_segment_index = platform.curr_segment_index;
        let curr_state = platform.curr_state;
        let current_position = Vec2::new((transform.translation.x / 8.0) - (platform.width as f32 / 2.0 / 8.0), 22.0 - (transform.translation.y / 8.0) + (platform.height as f32 / 2.0 / 8.0)); // NEED height of level and grid width
        let curr_direction = platform.curr_direction;
        
        let path_len = path.len() as i32;
        //println!("Path has {} segments. ", path_len); // testing
        let mut previous_segment_index = match curr_segment_index {
            Some(x) => (((x - 1) % path_len) + path_len) % path_len,
            None => -1 // Error index value
        };
        //println!("Next location: {:?}, current location: {:?} ", curr_segment, current_position); // testing

        let direction_vec = Vec2::new(curr_segment.unwrap().x as f32 - current_position.x, -(curr_segment.unwrap().y as f32 - current_position.y)).normalize();
        let direction_and_velocity = direction_vec * speed;
        platform.curr_velocity = Some(direction_and_velocity.clone());

        velocity.linvel = direction_and_velocity;


        let direction_vec_3d = Vec3::new(curr_segment.unwrap().x as f32 - current_position.x, -(curr_segment.unwrap().y as f32 - current_position.y), 0.0).normalize();
        transform.translation += direction_vec_3d * platform.speed * time.delta_secs();



        //impulse.impulse = direction_and_velocity;
        let distance = current_position.distance(curr_segment.unwrap().as_vec2());
        //if current_position.x as i32 == curr_segment.unwrap().x && current_position.y as i32 == curr_segment.unwrap().y {
        if distance <= 0.005 * platform.speed {
            //println!("Platform reached segment {:?}! ", curr_segment_index); // testing
            platform.curr_segment = Some(path[((platform.curr_segment_index.unwrap() + 1) % path_len) as usize]);
            previous_segment_index = platform.curr_segment_index.unwrap();
            platform.curr_segment_index = Some((previous_segment_index + 1) % path_len);
            //transform.translation.x = platform.initial_pos.unwrap().x as f32 + platform.curr_segment.unwrap().x as f32;
            //transform.translation.y = platform.initial_pos.unwrap().y as f32 + platform.curr_segment.unwrap().y as f32;
        }
        
        /* Changes segment if platform arrives at next state
        if current_position.x as i32 == curr_segment.unwrap().x && current_position.y as i32 == curr_segment.unwrap().y {
            println!("Platform reached segment {:?}! ", curr_segment_index); // testing
            platform.curr_segment = Some(path[((platform.curr_segment_index.unwrap() + 1) % path_len) as usize]);
            previous_segment_index = platform.curr_segment_index.unwrap();
            platform.curr_segment_index = Some((previous_segment_index + 1) % path_len);
            transform.translation.x = platform.curr_segment.unwrap().x as f32;
            transform.translation.y = platform.curr_segment.unwrap().y as f32;
        }
        */
        // Move platform towards the next segment
    }
}

pub fn adjust_player(
    mut player_q: Query<
        (
            Entity,
            &mut KinematicCharacterController,
            &KinematicCharacterControllerOutput,
            &mut PlayerMovement,
            &mut Transform
        ),
        With<PlayerMarker>,
    >,
    platform_q: Query<&mut MovingPlatform>,
    rapier_context: ReadDefaultRapierContext,
    time: Res<Time>,
    mut commands: Commands
) {
    for mut player in player_q.iter_mut() {
        if let Some((entity, intersection)) = rapier_context.cast_ray(
            Vec2::new(player.4.translation.x, player.4.translation.y - 16.0),
            Vec2::new(0.0, -1.0), 
            0.0, 
            true, 
            QueryFilter::default()
        ) {
            if let Ok(platform) = platform_q.get(entity) {
                //println!("Entity: {:?}, Intersection: {:?} ", entity, intersection);
                //let joint = FixedJointBuilder::new()
                    //.local_anchor1(Vec2::ZERO)
                    //.local_anchor2(Vec2::new(0.0,-1.0));
                //commands.entity(entity).insert(ImpulseJoint::new(player.0, joint));
                if platform.curr_velocity.unwrap().y < 0.0 && player.2.grounded {
                    //player.3.velocity = player.3.velocity.lerp(platform.curr_velocity.unwrap(), 0.1)
                    //player.3.velocity.x = platform.curr_velocity.unwrap().x * 0.0725;
                    //platform.curr_velocity.unwrap();
                    //player.4.translation += Vec3::new(platform.curr_velocity.unwrap().x * time.delta_secs(), platform.curr_velocity.unwrap().y * time.delta_secs(), 0.0);
                }
            }
        }
    }
}

pub fn reset_platforms(
    mut platform_q: Query<(&mut MovingPlatform, &mut Transform)>
) {
    for (mut platform, mut transform) in platform_q.iter_mut() {
        println!("{:?}", platform.path);
        //transform.translation = Vec3::new((platform.path[0].x as f32) / 8.0, (platform.path[0].y as f32) / 8.0, 0.0);
        transform.translation = Vec3::new((platform.path[0].x as f32 * 8.0) + (platform.width as f32 / 2.0), (22.0 * 8.0) - (platform.path[0].y as f32 * 8.0) + (platform.height as f32 / 2.0), 0.0);
        platform.curr_segment = Some(platform.path[1]); // Bad, requires at least 1 point on path
    }
}

/**
 * 
 */
pub fn on_platform_changed() {
    println!("Not implemented yet!")
}

//pub fn reset_platforms() {
//
//}