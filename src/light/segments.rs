use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use enum_map::EnumMap;

use super::{
    render::{LightMaterial, LightRenderData}, BlackRayComponent, LightBeamSource, LightColor, LightSegmentZMarker, LIGHT_SPEED
};
use crate::{
    level::sensor::LightSensor, lighting::LineLight2d, particle::spark::SparkExplosionEvent,
    shared::GroupLabel,
};

/// Marker [`Component`] used to query for light segments.
#[derive(Default, Component, Clone, Debug)]
pub struct LightSegment {
    pub color: LightColor,
}

/// [`Bundle`] used in the initialization of the [`LightSegmentCache`] to spawn segment entities.
#[derive(Bundle, Debug, Clone, Default)]
pub struct LightSegmentBundle {
    pub segment: LightSegment,
    pub mesh: Mesh2d,
    pub material: MeshMaterial2d<LightMaterial>,
    pub visibility: Visibility,
    pub transform: Transform,
    pub line_light: LineLight2d,
}

/// [`Resource`] used to store [`Entity`] handles to the light segments so they aren't added and
/// despawned every frame. See [`simulate_light_sources`] for details.
#[derive(Resource, Default)]
pub struct LightSegmentCache {
    segments: EnumMap<LightColor, Vec<Entity>>,
}

/// Local variable for [`simulate_light_sources`] used to store the handle to the audio SFX
pub struct LightBounceSfx {
    bounce: [Handle<AudioSource>; 3],
    reflect: [Handle<AudioSource>; 3],
}

impl FromWorld for LightBounceSfx {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        LightBounceSfx {
            bounce: [
                asset_server.load("sfx/light/light-bounce-1.wav"),
                asset_server.load("sfx/light/light-bounce-2.wav"),
                asset_server.load("sfx/light/light-bounce-3.wav"),
            ],
            reflect: [
                asset_server.load("sfx/light/light-bounce-1-reflect.wav"),
                asset_server.load("sfx/light/light-bounce-2-reflect.wav"),
                asset_server.load("sfx/light/light-bounce-3-reflect.wav"),
            ],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LightBeamIntersection {
    pub entity: Entity,
    pub point: Vec2,
    pub time: f32,
}

/// Stores information about the trajectory of a LightBeam
#[derive(Debug)]
pub struct LightBeamPlayback {
    pub intersections: Vec<LightBeamIntersection>,
    pub end_point: Option<Vec2>,
    pub elapsed_time: f32,
}

impl LightBeamPlayback {
    pub fn iter_points<'a>(
        &'a self,
        source: &'a LightBeamSource,
    ) -> impl Iterator<Item = Vec2> + 'a {
        std::iter::once(source.start_pos)
            .chain(
                self.intersections
                    .iter()
                    .map(|intersection| intersection.point),
            )
            .chain(self.end_point.iter().copied())
    }
}

#[derive(Default, Debug, Component)]
pub struct PrevLightBeamPlayback {
    pub intersections: Vec<Option<LightBeamIntersection>>,
}

pub fn play_light_beam(
    rapier_context: &mut RapierContext,
    source: &LightBeamSource,
    black_ray_qry: &Query<(Entity, &BlackRayComponent)>,
) -> LightBeamPlayback {
    let mut ray_pos = source.start_pos;
    let mut ray_dir = source.start_dir;
    let collision_groups = match source.color {
        LightColor::White => CollisionGroups::new(
            GroupLabel::WHITE_RAY,
            GroupLabel::TERRAIN | GroupLabel::PLATFORM | GroupLabel::LIGHT_SENSOR,
        ),
        LightColor::Black => CollisionGroups::new(
            GroupLabel::BLACK_RAY,
            GroupLabel::TERRAIN | GroupLabel::PLATFORM | GroupLabel::LIGHT_SENSOR,
        ),
        LightColor::Blue => CollisionGroups::new(
            GroupLabel::BLUE_RAY,
            GroupLabel::TERRAIN
                | GroupLabel::PLATFORM
                | GroupLabel::LIGHT_SENSOR
                | GroupLabel::WHITE_RAY
                | GroupLabel::BLACK_RAY,
        ),
        _ => CollisionGroups::new(
            GroupLabel::LIGHT_RAY,
            GroupLabel::TERRAIN
                | GroupLabel::PLATFORM
                | GroupLabel::LIGHT_SENSOR
                | GroupLabel::WHITE_RAY
                | GroupLabel::BLACK_RAY,
        ),
    };

    let mut ray_qry = QueryFilter::new().groups(collision_groups);
    let mut remaining_time = source.time_traveled;

    let mut playback = LightBeamPlayback {
        intersections: vec![],
        end_point: None,
        elapsed_time: 0.0,
    };

    for _ in 0..source.color.num_bounces() + 1 {
        let Some((entity, intersection)) =
            rapier_context.cast_ray_and_get_normal(ray_pos, ray_dir, remaining_time, true, ray_qry)
        else {
            let final_point = ray_pos + ray_dir * remaining_time;
            playback.elapsed_time += remaining_time;
            playback.end_point = Some(final_point);
            break;
        };

        // if inside something???
        if intersection.time_of_impact < 0.01 {
            break;
        }

        playback.elapsed_time += intersection.time_of_impact;
        remaining_time -= intersection.time_of_impact;

        playback.intersections.push(LightBeamIntersection {
            entity,
            point: intersection.point,
            time: playback.elapsed_time,
        });

        ray_pos = intersection.point;
        ray_dir = ray_dir.reflect(intersection.normal);
        ray_qry = ray_qry.exclude_collider(entity);

        let mut found_black_ray_collision:i32 = 0;
        for (found_entity, _) in black_ray_qry.iter() {
            if found_entity == entity {
                found_black_ray_collision = 1;
                break;
            }
        }
        if found_black_ray_collision == 1 {
            break;
        }
    }

    playback
}

#[derive(Default, Component)]
pub struct LightBeamPoints(Vec<Vec2>);

/// [`System`] that runs on [`Update`], calculating the [`Transform`] of light segments from the
/// corresponding [`LightBeamSource`]. Note that this calculation happens every frame, so instead of
/// rapidly spawning/despawning the entities, we spawn them and cache them in the
/// [`LightSegmentCache`], then modify their [`Visibility`] and [`Transform`]s.
///
/// If needed, optimization work can be done by recalculating only segments that are currently
/// changing (segments already "stabilized" usually won't move).
#[allow(clippy::too_many_arguments)]
pub fn simulate_light_sources(
    mut commands: Commands,
    mut q_light_sources: Query<(Entity, &mut LightBeamSource, &mut PrevLightBeamPlayback)>,
    q_black_ray: Query<(Entity, &BlackRayComponent)>,
    mut q_rapier: Query<&mut RapierContext>,
    mut q_light_sensor: Query<&mut LightSensor>,
    // used to tell if a collision was against a white beam (a different sound is played)
    q_segments: Query<&LightSegment, Without<LightSegmentZMarker>>,
    light_bounce_sfx: Local<LightBounceSfx>,
    mut ev_spark_explosion: EventWriter<SparkExplosionEvent>,
) {
    let Ok(rapier_context) = q_rapier.get_single_mut() else {
        return;
    };
    // Reborrow!!!
    let rapier_context = rapier_context.into_inner();

    for (source_entity, mut source, mut prev_playback) in q_light_sources.iter_mut() {
        let playback = play_light_beam(rapier_context, &source, &q_black_ray);

        let mut pts: Vec<Vec2> = playback.iter_points(&source).collect();

        let intersections = playback.intersections.len();
        for i in 0..intersections {
            let prev_x = prev_playback.intersections.get(i).cloned().flatten();
            let new_x = playback.intersections[i];

            let is_same_intersection = prev_x.is_some_and(|prev_x| prev_x.entity == new_x.entity);

            // diff intersection
            if !is_same_intersection {
                let is_closer = prev_x.is_none_or(|prev_x| prev_x.time > new_x.time);

                // remvoe all points after the current intersection
                pts.truncate(i + 2);

                let add_intersection = prev_x.is_none() || is_closer;
                let remove_intersection = prev_x.is_some();
                let play_sound = prev_x.is_none();

                // handle remove before add because it could be the case that both are true
                if remove_intersection {
                    pts[i + 1] = prev_x.unwrap().point;
                    if let Ok(mut sensor) = q_light_sensor.get_mut(prev_x.unwrap().entity) {
                        sensor.hit_by[source.color] = false;
                    }
                    prev_playback.intersections[i] = None;
                    source.time_traveled = prev_x.unwrap().time;
                }

                if add_intersection {
                    pts[i + 1] = new_x.point;
                    if let Ok(mut sensor) = q_light_sensor.get_mut(new_x.entity) {
                        sensor.hit_by[source.color] = true;
                    }
                    if i >= prev_playback.intersections.len() {
                        assert!(i == prev_playback.intersections.len());
                        prev_playback.intersections.push(Some(new_x));
                    } else {
                        prev_playback.intersections[i] = Some(new_x);
                    }
                    source.time_traveled = new_x.time;
                }

                if play_sound && source.color != LightColor::Black {
                    let reflect = match q_segments.get(new_x.entity) {
                        Ok(segment) => segment.color == LightColor::White,
                        _ => false,
                    };
                    let audio = if reflect {
                        light_bounce_sfx.reflect[i].clone()
                    } else {
                        light_bounce_sfx.bounce[i].clone()
                    };
                    ev_spark_explosion.send(SparkExplosionEvent {
                        pos: new_x.point,
                        color: source.color.light_beam_color(),
                    });
                    commands
                        .entity(new_x.entity)
                        .with_child((AudioPlayer::new(audio), PlaybackSettings::DESPAWN));
                }

                prev_playback.intersections.truncate(i + 1);
                break;
            } else {
                // keep on updating the previous intersection buffer because this could be a moving
                // platform
                prev_playback.intersections[i] = Some(new_x);
            }
        }
        commands.entity(source_entity).insert(LightBeamPoints(pts));
    }
}

pub fn spawn_needed_segments(
    mut commands: Commands,
    q_light_sources: Query<(&LightBeamSource, &LightBeamPoints)>,
    mut segment_cache: ResMut<LightSegmentCache>,
    light_render_data: Res<LightRenderData>,
    q_light_segment_z: Query<&Transform, With<LightSegmentZMarker>>,
) {
    let Ok(light_segment_z) = q_light_segment_z.get_single() else {
        return;
    };
    for (source, pts) in q_light_sources.iter() {
        let segments = pts.0.len() - 1;
        // lazily spawn segment entities until there are enough segments to display the light beam
        // path
        while segment_cache.segments[source.color].len() < segments.min(10) {
            let id = commands
                .spawn(LightSegmentBundle {
                    segment: LightSegment {
                        color: source.color,
                    },
                    mesh: light_render_data.mesh.clone(),
                    material: light_render_data.material_map[source.color].clone(),
                    visibility: Visibility::Hidden,
                    transform: Transform::from_xyz(0., 0., light_segment_z.translation.z),
                    line_light: LineLight2d {
                        color: source.color.lighting_color().extend(1.0),
                        half_length: 10.0,
                        radius: 20.0,
                        volumetric_intensity: 0.008,
                    },
                })
                .id();
            // White beams need colliders
            if source.color == LightColor::White {
                commands.entity(id).insert((
                    Collider::cuboid(0.5, 0.5),
                    Sensor,
                    CollisionGroups::new(
                        GroupLabel::WHITE_RAY,
                        GroupLabel::TERRAIN
                            | GroupLabel::PLATFORM
                            | GroupLabel::LIGHT_SENSOR
                            | GroupLabel::LIGHT_RAY
                            | GroupLabel::BLUE_RAY
                            | GroupLabel::BLACK_RAY,
                    ),
                ));
            }
            // Black beams need Black_Ray_Component and colliders
            if source.color == LightColor::Black {
                commands.entity(id).insert((BlackRayComponent, Sensor, Collider::cuboid(0.5, 0.5), CollisionGroups::new(GroupLabel::BLACK_RAY,
                    GroupLabel::TERRAIN
                        | GroupLabel::PLATFORM
                        | GroupLabel::LIGHT_SENSOR
                        | GroupLabel::LIGHT_RAY
                        | GroupLabel::BLUE_RAY
                        | GroupLabel::WHITE_RAY,
                )));
            }
            segment_cache.segments[source.color].push(id);
        }
    }
}

pub fn visually_sync_segments(
    q_light_sources: Query<(&LightBeamSource, &LightBeamPoints)>,
    segment_cache: Res<LightSegmentCache>,
    mut q_segments: Query<(&mut LineLight2d, &mut Transform, &mut Visibility), With<LightSegment>>,
) {
    for (source, pts) in q_light_sources.iter() {
        let pts = &pts.0;
        // use the light beam path to set the transform of the segments currently in the cache
        for (i, segment) in segment_cache.segments[source.color].iter().enumerate() {
            let Ok((mut line_light, mut c_transform, mut c_visibility)) =
                q_segments.get_mut(*segment)
            else {
                panic!("Segment doesn't have transform or visibility!");
            };
            if i + 1 < pts.len() && pts[i].distance(pts[i + 1]) > 0.1 {
                let midpoint = pts[i].midpoint(pts[i + 1]).extend(1.0);
                let scale = Vec3::new(pts[i].distance(pts[i + 1]), 1., 1.);
                let rotation = (pts[i + 1] - pts[i]).to_angle();

                let transform = Transform::from_translation(midpoint)
                    .with_scale(scale)
                    .with_rotation(Quat::from_rotation_z(rotation));

                line_light.half_length = scale.x / 2.0;
                *c_transform = transform;
                *c_visibility = Visibility::Visible;
            } else {
                // required for white beam
                line_light.half_length = 0.0;
                *c_transform = Transform::default();
                *c_visibility = Visibility::Hidden;
            }
        }
    }
}

/// [`System`] that runs on [`FixedUpdate`], advancing the distance the light beam can travel.
pub fn tick_light_sources(mut q_light_sources: Query<&mut LightBeamSource>) {
    for mut source in q_light_sources.iter_mut() {
        source.time_traveled += LIGHT_SPEED;
    }
}

/// [`System`] that is responsible for hiding all of the [`LightSegment`](LightSegmentBundle)s
/// and despawning [`LightBeamSource`]s when the level changes.
pub fn cleanup_light_sources(
    mut commands: Commands,
    q_light_sources: Query<(Entity, &LightBeamSource)>,
    segment_cache: Res<LightSegmentCache>,
    mut q_segments: Query<(&mut Transform, &mut Visibility), With<LightSegment>>,
) {
    // FIXME: should make these entities children of the level so that they are despawned
    // automagically (?)

    for (entity, light_beam_source) in q_light_sources.iter() {
        if light_beam_source.color != LightColor::Black {
            commands.entity(entity).despawn_recursive();
        }
    }

    segment_cache.segments.iter().for_each(|(_, items)| {
        for &entity in items.iter() {
            let (mut transform, mut visibility) = q_segments
                .get_mut(entity)
                .expect("Segment should have visibility");

            // required for white beam
            *transform = Transform::default();
            *visibility = Visibility::Hidden;
        }
    });
}
