use bevy::{ecs::entity::EntityHashSet, prelude::*, time::Stopwatch};
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;
use std::time::Duration;

use crate::{
    level::{crystal::{CrystalColor, CrystalToggleEvent}, platform::{self, ChangePlatformState, PlatformToggleEvent}},
    shared::GroupLabel,
};

use super::LightColor;

/// [`Event`] used to notify other entities to trigger based on collision with light.
/// An included [`Entity`] is used to indicate the corresponding [`LightSensor`].
#[derive(Event)]
pub struct HitByLightEvent(pub Entity);

/// [`Component`] added to entities receptive to light. The
/// [`activation_timer`](LightSensor::activation_timer) should be initialized in the
/// `From<&EntityInstance>` implemenation for the [`LightSensorBundle`], if not default.
#[derive(Component)]
pub struct LightSensor {
    /// Stores the cumulative time light has been hitting the sensor
    pub cumulative_exposure: Stopwatch,
    /// The amount of time the light beam needs to be hitting the sensor for activation
    pub activation_timer: Timer,
    /// Whether or not the sensor was hit the previous frame
    pub was_hit: bool,
    /// The color of the crystals to toggle
    pub toggle_color: CrystalColor,
    /// The id of the platform to toggle
    pub platform_id: i32
}

impl LightSensor {
    fn new(toggle_color: CrystalColor, platform_id: i32) -> Self {
        let mut timer = Timer::new(Duration::from_millis(300), TimerMode::Once);
        timer.pause();
        LightSensor {
            activation_timer: timer,
            cumulative_exposure: Stopwatch::default(),
            was_hit: false,
            toggle_color,
            platform_id
        }
    }
}

/// [`Bundle`] that includes all the [`Component`]s needed for a [`LightSensor`] to function
/// properly.
#[derive(Bundle)]
pub struct LightSensorBundle {
    collider: Collider,
    sensor: Sensor,
    collision_groups: CollisionGroups,
    light_sensor: LightSensor,
}

impl From<&EntityInstance> for LightSensorBundle {
    fn from(entity_instance: &EntityInstance) -> Self {
        match entity_instance.identifier.as_ref() {
            "Button" => {
                let light_color: LightColor = entity_instance
                    .get_enum_field("light_color")
                    .expect("light_color needs to be an enum field on all buttons")
                    .into();

                let id = entity_instance
                    .get_int_field("id")
                    .expect("id needs to be an int field on all buttons");

                let sensor_color = CrystalColor {
                    color: light_color,
                    id: *id,
                };

                //println!("id: {:?}, ", *id);

                let platform_id = match entity_instance.get_int_field("platform_id") {
                    Ok(platform_id) => *platform_id,
                    Err(_) => -1
                };

                //println!("platform id: {:?} ", platform_id);

                return Self {
                    collider: Collider::cuboid(4., 4.),
                    sensor: Sensor,
                    collision_groups: CollisionGroups::new(
                        GroupLabel::LIGHT_SENSOR,
                        GroupLabel::LIGHT_RAY | GroupLabel::WHITE_RAY,
                    ),
                    light_sensor: LightSensor::new(sensor_color, platform_id),
                };
            }
            _ => unreachable!(),
        }
    }
}

/// [`System`] that resets the [`LightSensor`]s when a [`LevelSwitchEvent`] is received.
pub fn reset_light_sensors(mut q_sensors: Query<&mut LightSensor>) {
    for mut sensor in q_sensors.iter_mut() {
        sensor.activation_timer.reset();
        sensor.activation_timer.pause();
        sensor.was_hit = false;
        sensor.cumulative_exposure.reset();
    }
}

/// [`System`] that runs on [`Update`], querying each [`LightSensor`] and updating them
/// based on each [`HitByLightEvent`] generated in the [`System`]:
/// [`simulate_light_sources`](crate::light::segments::simulate_light_sources). This design
/// is still imperfect, as while it differs semantically from the previous implementation,
/// each [`Event`] is generated every frame. Preferably, refactor to include a "yap"-free
/// implementation across multiple systems to better utilize [`Event`].
pub fn update_light_sensors(
    mut commands: Commands,
    mut q_sensors: Query<(Entity, &mut LightSensor)>,
    mut ev_hit_by_light: EventReader<HitByLightEvent>,
    mut ev_crystal_toggle: EventWriter<CrystalToggleEvent>,
    mut platform_change: EventWriter<ChangePlatformState>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    let mut hit_sensors: EntityHashSet = EntityHashSet::default();
    for ev in ev_hit_by_light.read() {
        hit_sensors.insert(ev.0);
    }

    for (entity, mut sensor) in q_sensors.iter_mut() {
        let was_hit = hit_sensors.contains(&entity);

        if was_hit {
            if !sensor.was_hit {
                sensor.activation_timer.unpause();
            }
            sensor.cumulative_exposure.tick(time.delta());
        }

        // if prev sensor state was different than current, we reset its timer
        if sensor.was_hit != was_hit {
            sensor.activation_timer.reset();
        }

        sensor.activation_timer.tick(time.delta());

        if sensor.activation_timer.just_finished() {
            ev_crystal_toggle.send(CrystalToggleEvent {
                color: sensor.toggle_color,
            });
            
            if was_hit {
                platform_change.send(ChangePlatformState::Play {
                    id: sensor.platform_id
                });
                println!("Activated Platform!");
            } else {
                platform_change.send(ChangePlatformState::Pause {
                    id: sensor.platform_id
                });
                println!("Deactivated Platform!");
            }

            commands.entity(entity).with_child((
                AudioPlayer::new(asset_server.load("sfx/button.wav")),
                PlaybackSettings::DESPAWN,
            ));
        }

        sensor.was_hit = was_hit;
    }
}
