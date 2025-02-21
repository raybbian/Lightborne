use std::{default, mem::discriminant};

use bevy::{prelude::*, text::cosmic_text::ttf_parser::loca, utils::hashbrown::hash_map::IterMut};
use bevy_ecs_ldtk::{ldtk::{ldtk_fields, FieldInstance}, prelude::*};
use bevy_rapier2d::prelude::*;
use bevy::log::LogPlugin;
use bevy::math;

use crate::{player::{PlayerBundle, PlayerMarker}, shared::{GroupLabel, ResetLevel}};

use super::{entity, LevelSystems};


/// Plugin for handling platforms
pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlatformToggleEvent>()
            .add_systems(PreUpdate,(initialize_platforms,).in_set(LevelSystems::Processing).run_if(on_event::<ResetLevel>))
            .add_systems(Update, on_platform_changed.in_set(LevelSystems::Simulation).run_if(on_event::<ResetLevel>))
            .add_systems(FixedUpdate, move_platforms.in_set(LevelSystems::Processing))
            .register_ldtk_entity::<MovingPlatformBundle>("MovingPlatform")
            .add_systems(Startup, initialize_platforms);
            //.add_systems(FixedUpdate, reset_platforms.run_if(on_event::<ResetLevel>)); // May be unnecessary if resetting on loading and resetting the same way
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
    pub curr_direction: Option<bool>
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
#[derive(Default, Bundle)]
pub struct PlatformPhysicsBundle {
    pub rigid_body: RigidBody,
    //pub external_impulse: ExternalImpulse,
    pub collider: Collider,
    //pub gravity_scale: GravityScale,
    // pub locked_axes: LockedAxes,
    pub velocity: Velocity
}

/// Bundle for moving platforms
#[derive(Default, Bundle, LdtkEntity)]
pub struct MovingPlatformBundle {
    #[from_entity_instance]
    pub platform: MovingPlatform,
    pub transform: Transform,
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
    mut level_q: Query<(&mut MovingPlatform, &mut Transform, &GlobalTransform, &mut RigidBody, &mut Collider)>,
    player_q: Query<&GlobalTransform, With<PlayerMarker>>,
    asset_server: Res<AssetServer>
) {
    for (mut platform, mut local_transform, global_transform, mut rigid_body, mut collider) in level_q.iter_mut() {
        // Initialization:

        if platform.curr_segment.is_none() {
            println!("Path is {:?} ", platform.path);
            println!("Default State is {:?} ", platform.default_state);
            platform.curr_segment = Some(platform.path[0]);
        }

        if platform.curr_segment_index.is_none() {
            platform.curr_segment_index = Some(1);
        }

        if platform.initial_pos.is_none() {
            let initial_position = Vec2::new(local_transform.translation.x.round(), local_transform.translation.y.round());
            //platform.initial_pos = Some(initial_position.clone());
            platform.initial_pos = Some(initial_position);
            platform.path.insert(0, IVec2::new(0, 0));
            for mut vector in &mut platform.path {
                *vector = *vector * 2;
            }
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
        *rigid_body = RigidBody::KinematicVelocityBased;
        *collider = Collider::cuboid(platform.width as f32 / 2.0, platform.height as f32 / 2.0);
        platform.is_init = Some(true);

        // Testing:

        /*
        println!("Entity found!");
        let speed = platform.speed;
        let path = (&platform.path).to_vec();
        let default_state = platform.default_state;
        let x = global_transform.translation().x;
        let y = global_transform.translation().y;
        println!("Current Position: x = {}, y = {}", x, y);
        let Ok(player) = player_q.get_single() else {
            return;
        };
        let xP = player.translation().x;
        let yP = player.translation().y;
        local_transform.translation.x += 16.0;
        println!("Transform: {} ", local_transform.translation);
        //println!("Current Player Position: x = {}, y = {}", xP, yP);
        */
        /*
        let platform_texture = asset_server.load("platform.png");
        commands.spawn(
            MovingPlatformBundle {
                platform: MovingPlatform {
                    speed,
                    path,
                    default_state,
                    id
                },
                sprite: Sprite {
                    color: Color::rgb(0.0, 1.0, 0.0),
                    flip_x: false,
                    flip_y: false,
                    custom_size: Default::default(),
                    rect: None,
                    image: platform_texture,
                    anchor: bevy::sprite::Anchor::Center,
                    texture_atlas: Default::default(),
                    image_mode: Default::default()
                },
                transform: Transform::default(),
                global_transform: GlobalTransform::default()
            }
        );
        */
    }
}

pub fn move_platforms(
    mut level_q: Query<(&mut MovingPlatform, &mut Transform, &mut RigidBody, &mut Velocity)>
) {
    for (mut platform, mut transform, mut rigid_body, mut velocity) in level_q.iter_mut() {
        if platform.is_init.is_none() {
            continue;
        }
        let path = platform.path.clone();
        print!("Path is {:?} ", platform.path);
        let speed = platform.speed;
        let curr_segment = platform.curr_segment.clone();
        let curr_segment_index = platform.curr_segment_index;
        let curr_state = platform.curr_state;
        let current_position = Vec2::new(transform.translation.x - platform.initial_pos.unwrap().x, transform.translation.y - platform.initial_pos.unwrap().y);
        let curr_direction = platform.curr_direction;
        
        let path_len = path.len() as i32;
        println!("Path has {} segments. ", path_len); // testing
        let mut previous_segment_index = match curr_segment_index {
            Some(x) => (((x - 1) % path_len) + path_len) % path_len,
            None => -1 // Error index value
        };
        println!("Current path segment: {:?}, Previous path segment: {} ", curr_segment_index, previous_segment_index); // testing

        let direction_vec = Vec2::new(curr_segment.unwrap().x as f32 - current_position.x, curr_segment.unwrap().y as f32 - current_position.y).normalize();
        let direction_and_velocity = direction_vec * speed;

        velocity.linvel = direction_and_velocity;
        //impulse.impulse = direction_and_velocity;
        let distance = current_position.distance(curr_segment.unwrap().as_vec2());
        //if current_position.x as i32 == curr_segment.unwrap().x && current_position.y as i32 == curr_segment.unwrap().y {
        if distance <= 0.1 {
            println!("Platform reached segment {:?}! ", curr_segment_index); // testing
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

/**
 * 
 */
pub fn on_platform_changed() {
    println!("Not implemented yet!")
}

//pub fn reset_platforms() {
//
//}





/*

Platforms have:
    * Position -> (x,y)
    * Path -> Vec<>
    * Speed -> (x)
    * Triggered/Continuous -> enum(Triggered/Continous)
    * Repeat -> boolean
    * DefaultActive
    * IsActive
*/