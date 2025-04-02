use std::time::Duration;

use bevy::{prelude::*, utils::HashMap};

use crate::{light::segments::LightSegment, particle::emitter::ParticleModifier};

use super::{
    ParticleBundle, ParticleEmitter, ParticleEmitterArea, ParticleEmitterOptions, ParticleOptions,
    ParticlePhysicsOptions,
};

#[derive(Resource, Default)]
pub struct SegmentTransformMap(HashMap<Entity, Transform>);

#[allow(clippy::type_complexity)]
pub fn add_segment_sparks(
    mut commands: Commands,
    mut transform_map: ResMut<SegmentTransformMap>,
    light_segment: Query<
        (Entity, &LightSegment, &Transform, Option<&mut Children>),
        Or<(Added<LightSegment>, Changed<Transform>)>,
    >,
    q_emitter: Query<&ParticleEmitter>,
    asset_server: Res<AssetServer>,
) {
    const VEL: f32 = 30.0;
    for (entity, segment, transform, children) in light_segment.iter() {
        if transform_map.0.get(&entity).is_some_and(|t| t == transform) {
            continue;
        }

        // remove and despawn all emitter children
        if let Some(children) = children {
            let emitter_children: Vec<_> = children
                .iter()
                .cloned()
                .filter(|&child| q_emitter.contains(child))
                .collect();
            commands.entity(entity).remove_children(&emitter_children);
            for child in emitter_children {
                commands.entity(child).despawn();
            }
        };

        commands
            .entity(entity)
            .insert_if_new(ParticleEmitter::new(ParticleEmitterOptions {
                area: ParticleEmitterArea::Capsule { radius: 1.0 },
                delay_range: Duration::from_secs_f32(0.0)..Duration::from_secs_f32(500.0),
                scale_delay_by_area: true,
                particles: vec![new_spark_particle(
                    segment.color.light_beam_color(),
                    &asset_server,
                )],
                modifier: ParticleModifier {
                    add_velocity: Some((-VEL..VEL, -VEL..VEL)),
                },
            }))
            .with_child((
                ParticleEmitter::new(ParticleEmitterOptions {
                    area: ParticleEmitterArea::Circle { radius: 0.5 },
                    delay_range: Duration::from_secs_f32(0.0)..Duration::from_secs_f32(0.4),
                    particles: vec![new_spark_particle(
                        segment.color.light_beam_color(),
                        &asset_server,
                    )],
                    modifier: ParticleModifier {
                        add_velocity: Some((-VEL..VEL, -VEL..VEL)),
                    },
                    ..default()
                }),
                Transform::from_translation(Vec3::new(0.5, 0.0, 0.0)), // moves child emitter to the end of segment (due to scale)
            ));
        transform_map.0.insert(entity, *transform);
    }
}

#[derive(Event)]
pub struct SparkExplosionEvent {
    pub pos: Vec2,
    pub color: Color,
}

pub fn create_spark_explosions(
    mut commands: Commands,
    mut spark_explosion_events: EventReader<SparkExplosionEvent>,
    asset_server: Res<AssetServer>,
) {
    const VEL: f32 = 50.0;
    let modifier: ParticleModifier = ParticleModifier {
        add_velocity: Some((-VEL..VEL, -VEL..VEL)),
    };
    for event in spark_explosion_events.read() {
        for _ in 0..15 {
            let SparkExplosionEvent { pos, color } = *event;
            let mut particle_options = new_spark_particle(color, &asset_server);
            modifier.modify(&mut particle_options);
            commands.spawn(ParticleBundle::new(particle_options, pos));
        }
    }
}

fn new_spark_particle(color: Color, asset_server: &Res<AssetServer>) -> ParticleOptions {
    ParticleOptions {
        life_time: Duration::from_secs_f32(0.6),
        physics: Some(ParticlePhysicsOptions {
            wind_mult: 0.0,
            gravity_mult: 200.0,
            starting_velocity: Vec2::new(0.0, 10.0),
        }),
        sprite: Sprite {
            image: asset_server.load("particle/spark.png"),
            color,
            ..default()
        },
        fade_away: true,
        ..default()
    }
}
