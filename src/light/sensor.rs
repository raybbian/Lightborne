use bevy::{prelude::*, time::Stopwatch};
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;
use std::time::Duration;

use crate::{
    level::crystal::{CrystalColor, CrystalToggleEvent},
    shared::GroupLabel,
};

use super::LightColor;

/// [`Component`] added to entities receptive to light. The
/// [`activation_timer`](LightSensor::activation_timer) should be initialized in the
/// `From<&EntityInstance>` implemenation for the [`LightSensorBundle`], if not default.
#[derive(Component)]
pub struct LightSensor {
    /// Stores the cumulative time light has been hitting the sensor
    pub cumulative_exposure: Stopwatch,
    /// The amount of time the light beam needs to be hitting the sensor for activation
    pub activation_timer: Timer,
    /// Number of light beams hitting the sensor
    pub hit_count: usize,
    /// The color of the crystals to toggle
    pub toggle_color: CrystalColor,
    /// If the sensor was previously hit or not
    pub was_hit: Option<bool>,
}

impl LightSensor {
    fn new(toggle_color: CrystalColor) -> Self {
        LightSensor {
            activation_timer: Timer::new(Duration::from_millis(300), TimerMode::Once),
            cumulative_exposure: Stopwatch::default(),
            hit_count: 0,
            was_hit: None,
            toggle_color,
        }
    }

    fn reset(&mut self) {
        self.activation_timer.reset();
        self.hit_count = 0;
        self.was_hit = None;
        self.cumulative_exposure.reset();
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

                Self {
                    collider: Collider::cuboid(4., 4.),
                    sensor: Sensor,
                    collision_groups: CollisionGroups::new(
                        GroupLabel::LIGHT_SENSOR,
                        GroupLabel::LIGHT_RAY | GroupLabel::WHITE_RAY | GroupLabel::BLUE_RAY,
                    ),
                    light_sensor: LightSensor::new(sensor_color),
                }
            }
            _ => unreachable!(),
        }
    }
}

/// [`System`] that resets the [`LightSensor`]s when a [`LevelSwitchEvent`] is received.
pub fn reset_light_sensors(mut q_sensors: Query<&mut LightSensor>) {
    for mut sensor in q_sensors.iter_mut() {
        sensor.reset()
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
    mut ev_crystal_toggle: EventWriter<CrystalToggleEvent>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    for (entity, mut sensor) in q_sensors.iter_mut() {
        let was_hit = sensor.hit_count > 0;

        if was_hit {
            sensor.cumulative_exposure.tick(time.delta());
        }

        match (was_hit, sensor.was_hit) {
            (_, None) => {
                sensor.activation_timer.pause();
            }
            (true, Some(false)) => {
                sensor.activation_timer.unpause();
                sensor.activation_timer.reset();
            }
            (false, Some(true)) => {
                sensor.activation_timer.reset();
            }
            _ => (),
        }

        sensor.was_hit = Some(was_hit);
        sensor.activation_timer.tick(time.delta());

        if sensor.activation_timer.just_finished() {
            ev_crystal_toggle.send(CrystalToggleEvent {
                color: sensor.toggle_color,
            });

            commands.entity(entity).with_child((
                AudioPlayer::new(asset_server.load("sfx/button.wav")),
                PlaybackSettings::DESPAWN,
            ));
        }
    }
}
