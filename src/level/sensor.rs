use bevy::{prelude::*, time::Stopwatch};
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;
use enum_map::EnumMap;

use crate::{
    level::{crystal::{CrystalIdent, CrystalToggleEvent}, platform::ChangePlatformStateEvent},
    light::segments::simulate_light_sources,
    lighting::LineLight2d,
};

use super::{crystal::CrystalColor, entity::FixedEntityBundle, LevelSystems, LightColor, platform::PlatformState};

pub struct LightSensorPlugin;

impl Plugin for LightSensorPlugin {
    fn build(&self, app: &mut App) {
        app.register_ldtk_entity::<LightSensorBundle>("Sensor")
            .add_systems(
                PreUpdate,
                add_sensor_sprites.in_set(LevelSystems::Processing),
            )
            .add_systems(Update, reset_light_sensors.in_set(LevelSystems::Reset))
            .add_systems(
                FixedUpdate,
                update_light_sensors
                    .after(simulate_light_sources)
                    .in_set(LevelSystems::Simulation),
            );
    }
}

/// [`Component`] added to entities receptive to light. The
/// [`activation_timer`](LightSensor::activation_timer) should be initialized in the
/// `From<&EntityInstance>` implemenation for the [`LightSensorBundle`], if not default.
///
/// The [`Sprite`] on the entity containing a [`LightSensor`] refers to the center part of the
/// sprite, which will be colored depending on the light that hits it.
#[derive(Component, Debug)]
pub struct LightSensor {
    /// Stores the cumulative time light has been hitting the sensor
    pub cumulative_exposure: Stopwatch,
    /// Stores the amount of light stored in the sensor, from 0 to 1.
    pub meter: f32,
    /// Colors of light beams hitting the sensor
    pub hit_by: EnumMap<LightColor, bool>,
    /// Active state of the sensor
    pub is_active: bool,
    /// The color of the crystals to toggle
    pub toggle_ident: CrystalIdent,
    /// Meter's rate of change, per fixed timestep tick.
    rate: f32,
    /// The id of the platform to toggle
    pub platform_id: i32,
    /// Stored color used to animate the center of the sensor when the light no longer hits it
    stored_color: Color,
}

impl LightSensor {
    fn new(toggle_color: CrystalColor, millis: i32, platform_id: i32) -> Self {
        let rate = 1.0 / (millis as f32) * (1000.0 / 64.0);
        LightSensor {
            meter: 0.0,
            cumulative_exposure: Stopwatch::default(),
            hit_by: EnumMap::default(),
            is_active: false,
            toggle_ident,
            rate,
            platform_id,
            stored_color: Color::WHITE,
        }
    }

    fn reset(&mut self) {
        self.meter = 0.0;
        self.hit_by = EnumMap::default();
        self.is_active = false;
        self.cumulative_exposure.reset();
    }

    fn is_hit(&self) -> bool {
        self.hit_by.iter().any(|(_, hit_by_color)| *hit_by_color)
    }

    fn iter_hit_color(&self) -> impl Iterator<Item = LightColor> + '_ {
        self.hit_by
            .iter()
            .filter_map(|(color, hit_by_color)| if *hit_by_color { Some(color) } else { None })
    }
}

impl From<&EntityInstance> for LightSensor {
    fn from(entity_instance: &EntityInstance) -> Self {
        let toggle_color: CrystalColor = entity_instance
            .get_enum_field("toggle_color")
            .expect("toggle_color needs to be an enum field on all sensors")
            .into();

        let id = entity_instance
            .get_int_field("id")
            .expect("id needs to be an int field on all sensors");

        let millis = *entity_instance
            .get_int_field("activation_time")
            .expect("activation_time needs to be a float field on all sensors");

        let toggle_ident = CrystalIdent {
            color: toggle_color,
            id: *id,
        };

        let platform_id = match entity_instance.get_int_field("platform_id") {
            Ok(platform_id) => *platform_id,
            Err(_) => -1
        };

        LightSensor::new(sensor_color, millis, platform_id)
    }
}

pub fn add_sensor_sprites(
    mut commands: Commands,
    q_sensors: Query<(Entity, &LightSensor), Added<LightSensor>>,
    asset_server: Res<AssetServer>,
) {
    if q_sensors.is_empty() {
        return;
    }

    let sensor_inner = asset_server.load("sensor/sensor_inner.png");
    let sensor_outer = asset_server.load("sensor/sensor_outer.png");
    let sensor_center = asset_server.load("sensor/sensor_center.png");

    let inner_sprite = Sprite::from_image(sensor_inner);
    let mut outer_sprite = Sprite::from_image(sensor_outer);
    let center_sprite = Sprite::from_image(sensor_center);

    for (entity, sensor) in q_sensors.iter() {
        outer_sprite.color = sensor.toggle_ident.color.button_color();
        commands
            .entity(entity)
            .with_children(|sensor| {
                sensor.spawn(inner_sprite.clone());
                sensor.spawn(outer_sprite.clone());
            })
            .insert(center_sprite.clone());
    }
}

/// [`Bundle`] that includes all the [`Component`]s needed for a [`LightSensor`] to function
/// properly.
#[derive(Bundle, LdtkEntity)]
pub struct LightSensorBundle {
    // #[sprite_sheet]
    // sprite_sheet: Sprite,
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
    let toggle_color: CrystalColor = entity_instance
        .get_enum_field("toggle_color")
        .expect("toggle_color needs to be an enum field on all sensors")
        .into();

    LineLight2d::point(
        toggle_color
            .button_color()
            .to_linear()
            .to_vec3()
            .extend(0.5),
        35.0,
        0.008,
    )
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
    mut q_sensors: Query<(Entity, &mut LightSensor, &mut Sprite)>,
    mut ev_crystal_toggle: EventWriter<CrystalToggleEvent>,
    mut platform_change: EventWriter<ChangePlatformStateEvent>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    for (entity, mut sensor, mut sprite) in q_sensors.iter_mut() {
        let was_hit = sensor.is_hit();

        if was_hit {
            sensor.cumulative_exposure.tick(time.delta());

            // if the sensor was hit, update the stored color for the sensor
            let mut col = Vec3::ZERO;
            for color in sensor.iter_hit_color() {
                col += color.lighting_color() * 0.5;
            }
            col += Vec3::splat(0.6);
            sensor.stored_color = Color::srgb(col.x, col.y, col.z);
        }

        let juice = if was_hit { sensor.rate } else { -sensor.rate };
        sensor.meter += juice;

        let mut send_toggle = || {
            ev_crystal_toggle.send(CrystalToggleEvent {
                color: sensor.toggle_ident,
            });
            if was_hit {
                platform_change.send(ChangePlatformStateEvent {
                    new_state: PlatformState::Play,
                    id: sensor.platform_id,
                });
            } else {
                platform_change.send(ChangePlatformStateEvent {
                    new_state: PlatformState::Pause,
                    id: sensor.platform_id,
                });
            }
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

        sprite.color = Color::WHITE.mix(&sensor.stored_color, sensor.meter);
    }
}
