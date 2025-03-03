use bevy::{prelude::*, time::Stopwatch};
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::{
    level::crystal::{CrystalColor, CrystalToggleEvent},
    lighting::LineLight2d,
};

use super::{entity::FixedEntityBundle, LightColor};

/// [`Component`] added to entities receptive to light. The
/// [`activation_timer`](LightSensor::activation_timer) should be initialized in the
/// `From<&EntityInstance>` implemenation for the [`LightSensorBundle`], if not default.
#[derive(Component, Debug)]
pub struct LightSensor {
    /// Stores the cumulative time light has been hitting the sensor
    pub cumulative_exposure: Stopwatch,
    /// Stores the amount of light stored in the sensor, from 0 to 1.
    pub meter: f32,
    /// Number of light beams hitting the sensor
    pub hit_count: usize,
    /// Active state of the sensor
    pub is_active: bool,
    /// The color of the crystals to toggle
    pub toggle_color: CrystalColor,
    /// Meter's rate of change, per fixed timestep tick.
    rate: f32,
}

impl LightSensor {
    fn new(toggle_color: CrystalColor, millis: i32) -> Self {
        let rate = 1.0 / (millis as f32) * (1000.0 / 64.0);
        LightSensor {
            meter: 0.0,
            cumulative_exposure: Stopwatch::default(),
            hit_count: 0,
            is_active: false,
            toggle_color,
            rate,
        }
    }

    fn reset(&mut self) {
        self.meter = 0.0;
        self.hit_count = 0;
        self.is_active = false;
        self.cumulative_exposure.reset();
    }
}

impl From<&EntityInstance> for LightSensor {
    fn from(entity_instance: &EntityInstance) -> Self {
        let light_color: LightColor = entity_instance
            .get_enum_field("light_color")
            .expect("light_color needs to be an enum field on all buttons")
            .into();

        let id = entity_instance
            .get_int_field("id")
            .expect("id needs to be an int field on all buttons");

        let millis = *entity_instance
            .get_int_field("activation_time")
            .expect("activation_time needs to be a float field on all sensors");

        let sensor_color = CrystalColor {
            color: light_color,
            id: *id,
        };

        LightSensor::new(sensor_color, millis)
    }
}

pub fn color_sensors(mut q_buttons: Query<(&mut Sprite, &LightSensor), Added<LightSensor>>) {
    for (mut sprite, sensor) in q_buttons.iter_mut() {
        sprite.color = sensor.toggle_color.color.button_color();
    }
}

/// [`Bundle`] that includes all the [`Component`]s needed for a [`LightSensor`] to function
/// properly.
#[derive(Bundle, LdtkEntity)]
pub struct LightSensorBundle {
    #[sprite_sheet]
    sprite_sheet: Sprite,
    #[from_entity_instance]
    physics: FixedEntityBundle,
    #[default]
    sensor: Sensor,
    #[from_entity_instance]
    light_sensor: LightSensor,
    #[with(sensor_point_light)]
    lighting: LineLight2d,
}

pub fn sensor_point_light(entity_instance: &EntityInstance) -> LineLight2d {
    let light_color: LightColor = entity_instance
        .get_enum_field("light_color")
        .expect("light_color needs to be an enum field on all buttons")
        .into();

    LineLight2d::point(light_color.lighting_color().extend(0.5), 35.0, 0.008)
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

        let juice = if was_hit { sensor.rate } else { -sensor.rate };
        sensor.meter += juice;

        let mut send_toggle = || {
            ev_crystal_toggle.send(CrystalToggleEvent {
                color: sensor.toggle_color,
            });
            commands.entity(entity).with_child((
                AudioPlayer::new(asset_server.load("sfx/button.wav")),
                PlaybackSettings::DESPAWN,
            ));
        };

        if sensor.meter > 1.0 {
            if !sensor.is_active {
                send_toggle();
                sensor.is_active = true;
            }
            sensor.meter = 1.0;
        } else if sensor.meter < 0.0 {
            if sensor.is_active {
                send_toggle();
                sensor.is_active = false;
            }
            sensor.meter = 0.0;
        }
    }
}
