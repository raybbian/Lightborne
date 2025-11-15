use avian2d::prelude::*;
use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::{
    camera::HIGHRES_LAYER,
    game::{
        defs::mirror::Mirror,
        light::{
            render::{LightMaterial, LightRenderData},
            HitByLight, LightBeamSource, LightColor, LIGHT_SPEED,
        },
        lighting::LineLight2d,
        lyra::beam::PlayerLightInventory,
        particle::spark::SparkExplosionEvent,
        Layers,
    },
    shared::ResetLevels,
};

/// Marker [`Component`] used to query for light segments.
#[derive(Default, Component, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LightSegment {
    pub color: LightColor,
    pub index: usize,
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct LightSegmentCache(HashMap<LightSegment, (Transform, Entity, Entity)>);

/// [`Bundle`] used in the initialization of the [`LightSegmentCache`] to spawn segment entities.
#[derive(Bundle, Debug, Clone, Default)]
pub struct LightSegmentBundle {
    pub segment: LightSegment,
    pub mesh: Mesh2d,
    pub material: MeshMaterial2d<LightMaterial>,
    pub visibility: Visibility,
    pub transform: Transform,
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct LightBounceSfx {
    #[dependency]
    bounce: [Handle<AudioSource>; 3],
    #[dependency]
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
    pub intersections: Vec<LightBeamIntersection>,
}

#[derive(Component)]
pub struct LightBeamSourceDespawn;

const LIGHT_MAX_SEGMENTS: usize = 15;

pub fn play_light_beam(
    spatial_query: &SpatialQuery,
    source: &LightBeamSource,
    // black_ray_qry: &Query<(Entity, &BlackRayComponent)>,
    q_mirrors: &Query<&Mirror>,
) -> LightBeamPlayback {
    let mut ray_pos = source.start_pos;
    let mut ray_dir = source.start_dir;
    let collision_groups = match source.color {
        LightColor::White => CollisionLayers::new(
            Layers::WhiteRay,
            [
                Layers::Terrain,
                Layers::LightSensor,
                Layers::Spike,
                Layers::BlueCrystal,
                Layers::Platform,
            ],
        ),
        // LightColor::Black => {
        //     CollisionLayers::new(Layers::BlackRay, Layers::Terrain, Layers::LightSensor)
        // }
        LightColor::Blue => CollisionLayers::new(
            Layers::BlueRay,
            [
                Layers::Terrain,
                Layers::LightSensor,
                Layers::WhiteRay,
                Layers::Spike,
                Layers::Platform,
            ],
        ),
        _ => CollisionLayers::new(
            Layers::LightRay,
            [
                Layers::Terrain,
                Layers::LightSensor,
                Layers::WhiteRay,
                Layers::Spike,
                Layers::BlueCrystal,
                Layers::Platform,
            ],
        ),
    };

    let mut ray_qry = SpatialQueryFilter::default().with_mask(collision_groups.filters);
    let mut remaining_time = source.time_traveled;

    let mut playback = LightBeamPlayback {
        intersections: vec![],
        end_point: None,
        elapsed_time: 0.0,
    };

    let num_segments = source.color.num_bounces() + 1;

    let mut i = 0;
    let mut extra_bounces_from_mirror = 0;
    while i < num_segments + extra_bounces_from_mirror && i < LIGHT_MAX_SEGMENTS {
        let Some(hit) = spatial_query.cast_ray(ray_pos, ray_dir, remaining_time, true, &ray_qry)
        else {
            let final_point = ray_pos + ray_dir * remaining_time;
            playback.elapsed_time += remaining_time;
            playback.end_point = Some(final_point);
            break;
        };
        if q_mirrors.contains(hit.entity) {
            extra_bounces_from_mirror += 1;
        }

        // if inside something???
        let mut ignore_entity = true;
        if hit.distance < 0.01 {
            ignore_entity = false;
        }

        playback.elapsed_time += hit.distance;
        remaining_time -= hit.distance;
        let hit_point = ray_pos + *ray_dir * hit.distance;

        playback.intersections.push(LightBeamIntersection {
            entity: hit.entity,
            point: hit_point,
            time: playback.elapsed_time,
        });

        ray_pos = hit_point;
        ray_dir =
            Dir2::new((Vec2::from(ray_dir)).reflect(hit.normal)).expect("cast dir cannot be 0");
        if ignore_entity {
            ray_qry = ray_qry.with_excluded_entities([hit.entity]);
        }

        // if black_ray_qry.get(hit.entity).is_ok() {
        //     break;
        // }
        i += 1;
    }

    playback
}

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
    mut q_light_sources: Query<(&mut LightBeamSource, &mut PrevLightBeamPlayback)>,
    // q_black_ray: Query<(Entity, &BlackRayComponent)>,
    spatial_query: SpatialQuery,
    q_mirrors: Query<&Mirror>,
    // used to tell if a collision was against a white beam (a different sound is played)
    q_segments: Query<&LightSegment>,
    light_bounce_sfx: Res<LightBounceSfx>,
    mut ev_spark_explosion: MessageWriter<SparkExplosionEvent>,
    light_render_data: Res<LightRenderData>,
    mut segment_cache: ResMut<LightSegmentCache>,
) {
    let mut all_segments = segment_cache
        .iter()
        .map(|(k, _)| k.clone())
        .collect::<HashSet<LightSegment>>();

    for (mut source, mut prev_playback) in q_light_sources.iter_mut() {
        let playback = play_light_beam(&spatial_query, &source, &q_mirrors);
        let mut pts: Vec<Vec2> = playback.iter_points(&source).collect();
        source.time_traveled = playback.elapsed_time;

        let mut i = 0;
        loop {
            let prev_x = prev_playback.intersections.get(i).cloned();
            let new_x = playback.intersections.get(i).cloned();

            let is_same_intersection = match (prev_x, new_x) {
                (Some(px), Some(nx)) => px.entity == nx.entity,
                (None, None) => break,
                _ => false,
            };

            // diff intersection
            if !is_same_intersection {
                let is_closer = match (prev_x, new_x) {
                    (None, Some(_)) => true,
                    (Some(px), Some(nx)) => px.time > nx.time,
                    (Some(_), None) => false,
                    _ => break,
                };

                // remvoe all points after the current intersection
                pts.truncate(i + 2);

                let add_intersection = prev_x.is_none() || is_closer;
                let remove_intersection = prev_x.is_some();
                let play_sound = prev_x.is_none() && !is_same_intersection;

                // handle remove before add because it could be the case that both are true
                if remove_intersection {
                    let prev_x = prev_x.unwrap();
                    pts[i + 1] = prev_x.point;
                    source.time_traveled = prev_x.time;

                    // unhit current + future lights
                    for j in i..prev_playback.intersections.len() {
                        commands.trigger(HitByLight {
                            entity: prev_playback.intersections[j].entity,
                            color: source.color,
                            hit: false,
                        });
                    }
                    //leaves [0, i-1]
                    prev_playback.intersections.truncate(i);
                }

                if add_intersection {
                    let new_x = new_x.unwrap();
                    pts[i + 1] = new_x.point;
                    source.time_traveled = new_x.time;

                    commands.trigger(HitByLight {
                        entity: new_x.entity,
                        color: source.color,
                        hit: true,
                    });
                    assert!(i == prev_playback.intersections.len());
                    prev_playback.intersections.push(new_x);
                }

                if play_sound {
                    let new_x = new_x.unwrap();
                    let reflect = match q_segments.get(new_x.entity) {
                        Ok(segment) => segment.color == LightColor::White,
                        _ => false,
                    };
                    let audio = if reflect {
                        light_bounce_sfx
                            .reflect
                            .get(i)
                            .unwrap_or(&light_bounce_sfx.reflect[2])
                    } else {
                        light_bounce_sfx
                            .bounce
                            .get(i)
                            .unwrap_or(&light_bounce_sfx.bounce[2])
                    }
                    .clone();
                    ev_spark_explosion.write(SparkExplosionEvent {
                        pos: new_x.point,
                        color: source.color.light_beam_color(),
                    });
                    commands
                        .entity(new_x.entity)
                        .with_child((AudioPlayer::new(audio), PlaybackSettings::DESPAWN));
                }

                break;
            } else {
                // keep on updating the previous intersection buffer because this could be a moving
                // platform
                prev_playback.intersections[i] = new_x.unwrap();
            }
            i += 1;
        }

        for i in 0..pts.len() - 1 {
            if pts[i].distance(pts[i + 1]) < 0.1 {
                continue;
            }
            // NOTE: hardcode here should be okay
            let midpoint = pts[i].midpoint(pts[i + 1]).extend(4.);
            let scale = Vec3::new(pts[i].distance(pts[i + 1]), 1., 1.);
            let rotation = (pts[i + 1] - pts[i]).to_angle();

            let transform = Transform::from_translation(midpoint)
                .with_scale(scale)
                .with_rotation(Quat::from_rotation_z(rotation));

            let segment = LightSegment {
                color: source.color,
                index: i,
            };

            let (entity, light_entity) = match segment_cache.get(&segment) {
                None => {
                    let seg = commands
                        .spawn(transform)
                        .insert(LightSegmentBundle {
                            segment: LightSegment {
                                color: source.color,
                                index: i,
                            },
                            mesh: light_render_data.mesh.clone(),
                            material: light_render_data.material_map[source.color].clone(),
                            visibility: Visibility::Visible,
                            transform,
                        })
                        .insert(HIGHRES_LAYER)
                        .id();

                    let light = commands
                        .spawn(LineLight2d {
                            color: source.color.lighting_color().extend(1.0),
                            half_length: scale.x / 2.0,
                            radius: 20.0,
                            volumetric_intensity: 0.04,
                        })
                        .insert(ChildOf(seg))
                        .id();

                    if source.color == LightColor::White {
                        commands.entity(seg).insert((
                            Collider::rectangle(1., 1.),
                            Sensor,
                            CollisionLayers::new(
                                Layers::WhiteRay,
                                [Layers::LightRay, Layers::BlueRay],
                            ),
                        ));
                    }
                    all_segments.remove(&segment);
                    (seg, light)
                }
                Some((t, e, le)) => {
                    all_segments.remove(&segment);
                    if *t == transform {
                        continue;
                    }
                    commands.entity(*le).try_insert(LineLight2d {
                        color: source.color.lighting_color().extend(1.0),
                        half_length: scale.x / 2.0,
                        radius: 20.0,
                        volumetric_intensity: 0.04,
                    });
                    commands.entity(*e).try_insert(transform);
                    (*e, *le)
                }
            };
            segment_cache.insert(segment.clone(), (transform, entity, light_entity));
        }
    }

    for segment in all_segments {
        commands.entity(segment_cache[&segment].1).despawn();
        segment_cache.remove(&segment);
    }
}

pub fn tick_light_sources(
    mut commands: Commands,
    mut q_light_sources: Query<(Entity, &mut LightBeamSource, Has<LightBeamSourceDespawn>)>,
    mut lyra: Single<&mut PlayerLightInventory>,
    time: Res<Time>,
) {
    for (entity, mut source, despawning) in q_light_sources.iter_mut() {
        if despawning {
            source.time_traveled -= LIGHT_SPEED * time.delta_secs() * 64.;
            if source.time_traveled <= 0.0 {
                commands.entity(entity).despawn();
                lyra.collectible[source.color] = None;
            }
        } else {
            source.time_traveled += LIGHT_SPEED * time.delta_secs() * 64.;
        }
    }
}

pub fn cleanup_light_sources(
    _: On<ResetLevels>,
    mut commands: Commands,
    q_light_sources: Query<Entity, With<LightBeamSource>>,
    q_segments: Query<Entity, With<LightSegment>>,
    mut cache: ResMut<LightSegmentCache>,
) {
    for e in q_light_sources.iter() {
        commands.entity(e).despawn();
    }
    for e in q_segments.iter() {
        commands.entity(e).despawn();
    }
    cache.clear();
}
