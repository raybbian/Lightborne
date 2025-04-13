use rand::prelude::IndexedRandom;
use std::{f32::consts::PI, ops::Range, time::Duration};

use bevy::prelude::*;

use super::{ParticleBundle, ParticleOptions};

#[derive(Clone, Debug)]
pub enum ParticleEmitterArea {
    // Point,
    Cuboid { half_x: f32, half_y: f32 },
    Circle { radius: f32 },
    Capsule { radius: f32 },
}

impl ParticleEmitterArea {
    fn area(&self, scale: Vec3) -> f32 {
        match self {
            Self::Cuboid { half_x, half_y } => (2.0 * half_x) * (2.0 * half_y),
            Self::Capsule { radius } => scale.x.abs() * (2.0 * radius) + PI * radius.powi(2),
            Self::Circle { radius } => PI * radius.powi(2),
        }
    }
}

impl Default for ParticleEmitterArea {
    fn default() -> Self {
        ParticleEmitterArea::Circle { radius: 1.0 }
    }
}

#[derive(Clone, Debug)]
pub struct ParticleEmitterOptions {
    pub area: ParticleEmitterArea,
    pub particles: Vec<ParticleOptions>,
    pub delay_range: Range<Duration>,
    pub scale_delay_by_area: bool,
    pub modifier: ParticleModifier,
}

impl Default for ParticleEmitterOptions {
    fn default() -> Self {
        Self {
            area: ParticleEmitterArea::default(),
            particles: vec![ParticleOptions::default()],
            delay_range: Duration::from_secs(0)..Duration::from_secs(1),
            scale_delay_by_area: false,
            modifier: ParticleModifier::default(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ParticleModifier {
    pub add_velocity: Option<(Range<f32>, Range<f32>)>,
}

impl ParticleModifier {
    pub fn modify(&self, options: &mut ParticleOptions) {
        if let Some(ref mut physics) = options.physics {
            if let Some(add_velocity) = &self.add_velocity {
                physics.starting_velocity += Vec2::new(
                    rand::random_range(add_velocity.0.clone()),
                    rand::random_range(add_velocity.1.clone()),
                )
            }
        }
    }
}

#[derive(Component, Clone, Debug)]
#[require(Transform, Visibility)]
pub struct ParticleEmitter {
    pub options: ParticleEmitterOptions,
    pub timer: Timer,
}

impl ParticleEmitter {
    pub fn new(options: ParticleEmitterOptions) -> Self {
        Self {
            timer: Timer::new(
                rand::random_range(options.delay_range.clone()),
                TimerMode::Once,
            ),
            options,
        }
    }

    pub fn get_delay_range(&self, scale: Vec3) -> Range<Duration> {
        if self.options.scale_delay_by_area {
            let start = self.options.delay_range.start;
            let end = self.options.delay_range.end;
            let emitter_area = self.options.area.area(scale);
            if emitter_area > 0.5 {
                (Duration::from_secs_f32(start.as_secs_f32() / emitter_area))
                    ..(Duration::from_secs_f32(end.as_secs_f32() / emitter_area))
            } else {
                Duration::from_secs_f32(10_000.0)..Duration::from_secs_f32(10_001.0)
            }
        } else {
            self.options.delay_range.clone()
        }
    }
}

pub fn update_particle_emitters(
    mut commands: Commands,
    time: Res<Time>,
    mut emitters: Query<(&GlobalTransform, &InheritedVisibility, &mut ParticleEmitter)>,
) {
    for (transform, visibility, mut emitter) in emitters.iter_mut() {
        if *visibility == InheritedVisibility::HIDDEN {
            continue;
        }
        emitter.timer.tick(time.delta());
        if !emitter.timer.finished()
            && emitter.timer.elapsed() < emitter.get_delay_range(transform.scale()).end
        // make emitter does not wait for more than max delay range to emit next particle.
        // useful for emitters with changing areas, such as segment sparks.
        {
            continue;
        }
        emitter.timer = Timer::new(
            rand::random_range(emitter.get_delay_range(transform.scale())),
            TimerMode::Once,
        );
        let offset = match emitter.options.area {
            ParticleEmitterArea::Cuboid { half_x, half_y } => Vec2::new(
                half_x * rand::random_range(-1.0..1.0),
                half_y * rand::random_range(-1.0..1.0),
            ),
            ParticleEmitterArea::Capsule { radius } => {
                let unit_vec = transform
                    .rotation()
                    .mul_vec3(Vec3::new(1.0, 0.0, 0.0))
                    .truncate();
                let point_1_offset = unit_vec * transform.scale().x / 2.;
                let point_2_offset = -unit_vec * transform.scale().x / 2.;

                let weight = rand::random_range(0.0..1.0);
                let point_on_line = point_1_offset * weight + point_2_offset * (1.0 - weight);

                let angle: f32 = rand::random_range(0.0..(2.0 * PI));
                let dist = rand::random_range(0.0..radius);
                point_on_line + Vec2::new(angle.cos() * dist, angle.sin() * dist)
            }
            ParticleEmitterArea::Circle { radius } => {
                let angle: f32 = rand::random_range(0.0..(2.0 * PI));
                let dist = rand::random_range(0.0..radius);
                Vec2::new(angle.cos() * dist, angle.sin() * dist)
            }
        };
        let start_pos = transform.translation().truncate() + offset;
        let mut particle_options = emitter
            .options
            .particles
            .choose(&mut rand::rng())
            .expect("ParticleBundle particles were empty")
            .clone();
        emitter.options.modifier.modify(&mut particle_options);
        commands.spawn(ParticleBundle::new(particle_options, start_pos));
    }
}
