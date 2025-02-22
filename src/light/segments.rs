use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use enum_map::EnumMap;

use super::{
    render::{LightMaterial, LightRenderData},
    sensor::{HitByLightEvent, LightSensor},
    LightColor, LightRaySource, LIGHT_SPEED,
};
use crate::{lighting::light::LineLighting, shared::GroupLabel};

/// Marker [`Component`] used to query for light segments.
#[derive(Default, Component, Clone, Debug)]
pub struct LightSegmentMarker;

/// [`Bundle`] used in the initialization of the [`LightSegmentCache`] to spawn segment entities.
#[derive(Bundle, Debug, Default, Clone)]
pub struct LightSegmentBundle {
    pub marker: LightSegmentMarker,
    pub mesh: Mesh2d,
    pub material: MeshMaterial2d<LightMaterial>,
    pub visibility: Visibility,
    pub transform: Transform,
    pub line_light: LineLighting,
}

/// [`Resource`] used to store [`Entity`] handles to the light segments so they aren't added and
/// despawned every frame. See [`simulate_light_sources`] for details.
#[derive(Resource)]
pub struct LightSegmentCache {
    table: EnumMap<LightColor, Vec<Entity>>,
}

impl FromWorld for LightSegmentCache {
    fn from_world(world: &mut World) -> Self {
        let mut cache = LightSegmentCache {
            table: EnumMap::default(),
        };
        let render_data = world.resource::<LightRenderData>();

        let mut segment_bundles: EnumMap<LightColor, LightSegmentBundle> = EnumMap::default();

        for (color, _) in cache.table.iter_mut() {
            segment_bundles[color] = LightSegmentBundle {
                marker: LightSegmentMarker,
                mesh: render_data.mesh.clone(),
                material: render_data.material_map[color].clone(),
                visibility: Visibility::Hidden,
                transform: Transform::default(),
                line_light: LineLighting {
                    radius: 20.0,
                    color: color.lighting_color(),
                },
            }
        }

        for (color, segments) in cache.table.iter_mut() {
            while segments.len() < color.num_bounces() + 1 {
                let mut cmds = world.spawn(());
                cmds.insert(segment_bundles[color].clone());

                // White beams need colliders
                if color == LightColor::White {
                    cmds.insert((
                        Collider::cuboid(0.5, 0.5),
                        Sensor,
                        CollisionGroups::new(
                            GroupLabel::WHITE_RAY,
                            GroupLabel::TERRAIN
                                | GroupLabel::LIGHT_SENSOR
                                | GroupLabel::LIGHT_RAY
                                | GroupLabel::BLUE_RAY,
                        ),
                    ));
                }

                segments.push(cmds.id());
            }
        }

        cache
    }
}

/// Local variable for [`simulate_light_sources`] used to store the handle to the audio SFX
pub struct LightBounceSfx([Handle<AudioSource>; 3]);

impl FromWorld for LightBounceSfx {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        LightBounceSfx([
            asset_server.load("sfx/light/light-bounce-1.wav"),
            asset_server.load("sfx/light/light-bounce-2.wav"),
            asset_server.load("sfx/light/light-bounce-3.wav"),
        ])
    }
}

/// [`System`] that runs on [`Update`], calculating the [`Transform`] of light segments from the
/// corresponding [`LightRaySource`]. Note that this calculation happens every frame, so instead of
/// rapidly spawning/despawning the entities, we spawn them and cache them in the
/// [`LightSegmentCache`], then modify their [`Visibility`] and [`Transform`]s.
///
/// If needed, optimization work can be done by recalculating only segments that are currently
/// changing (segments already "stabilized" usually won't move).
///
/// Similar logic is duplicated in [`preview_light_path`](crate::player::light::preview_light_path),
/// these two systems should be merged.
#[allow(clippy::too_many_arguments)]
pub fn simulate_light_sources(
    mut commands: Commands,
    mut q_light_sources: Query<&mut LightRaySource>,
    mut q_rapier: Query<&mut RapierContext>,
    mut ev_hit_by_light: EventWriter<HitByLightEvent>,
    q_light_sensor: Query<&LightSensor>,
    mut q_segments: Query<(&mut Transform, &mut Visibility), With<LightSegmentMarker>>,
    segment_cache: Res<LightSegmentCache>,
    light_bounce_sfx: Local<LightBounceSfx>,
) {
    let Ok(rapier_context) = q_rapier.get_single_mut() else {
        return;
    };

    for mut source in q_light_sources.iter_mut() {
        let mut ray_pos = source.start_pos;
        let mut ray_dir = source.start_dir;
        let collision_groups = match source.color {
            LightColor::White => CollisionGroups::new(
                GroupLabel::WHITE_RAY,
                GroupLabel::TERRAIN | GroupLabel::LIGHT_SENSOR,
            ),
            LightColor::Blue => CollisionGroups::new(
                GroupLabel::BLUE_RAY,
                GroupLabel::TERRAIN | GroupLabel::LIGHT_SENSOR | GroupLabel::WHITE_RAY,
            ),
            _ => CollisionGroups::new(
                GroupLabel::LIGHT_RAY,
                GroupLabel::TERRAIN | GroupLabel::LIGHT_SENSOR | GroupLabel::WHITE_RAY,
            ),
        };

        let mut ray_qry = QueryFilter::new().groups(collision_groups);

        let mut pts: Vec<Vec2> = vec![ray_pos];
        let mut remaining_time = source.time_traveled;
        let mut bounces = 0;
        for _ in 0..source.color.num_bounces() + 1 {
            let Some((entity, intersection)) = rapier_context.cast_ray_and_get_normal(
                ray_pos,
                ray_dir,
                remaining_time,
                true,
                ray_qry,
            ) else {
                let final_point = ray_pos + ray_dir * remaining_time;
                pts.push(final_point);
                break;
            };

            if intersection.time_of_impact < 0.01 {
                break;
            }

            remaining_time -= intersection.time_of_impact;

            pts.push(intersection.point);

            bounces += 1;
            if bounces > source.num_bounces {
                source.num_bounces = bounces;
                // Add sound effects as child because this current entity could have been hit again
                commands.entity(entity).with_child((
                    AudioPlayer::new(light_bounce_sfx.0[bounces - 1].clone()),
                    PlaybackSettings::DESPAWN,
                ));
            }

            if q_light_sensor.get(entity).is_ok() {
                ev_hit_by_light.send(HitByLightEvent(entity));
            };

            ray_pos = intersection.point;
            ray_dir = ray_dir.reflect(intersection.normal);
            ray_qry = ray_qry.exclude_collider(entity);
        }

        for (i, segment) in segment_cache.table[source.color].iter().enumerate() {
            let Ok((mut c_transform, mut c_visibility)) = q_segments.get_mut(*segment) else {
                panic!("Segment did not have visibility or transform");
            };

            if i + 1 < pts.len() {
                let midpoint = pts[i].midpoint(pts[i + 1]).extend(1.0);
                let scale = Vec3::new(pts[i].distance(pts[i + 1]), 1., 1.);
                let rotation = (pts[i + 1] - pts[i]).to_angle();

                let transform = Transform::from_translation(midpoint)
                    .with_scale(scale)
                    .with_rotation(Quat::from_rotation_z(rotation));

                *c_transform = transform;
                *c_visibility = Visibility::Visible;
            } else {
                // required for white beam
                *c_transform = Transform::default();
                *c_visibility = Visibility::Hidden;
            }
        }
    }
}

/// [`System`] that runs on [`FixedUpdate`], advancing the distance the light beam can travel.
pub fn tick_light_sources(mut q_light_sources: Query<&mut LightRaySource>) {
    for mut source in q_light_sources.iter_mut() {
        source.time_traveled += LIGHT_SPEED;
    }
}

/// [`System`] that is responsible for hiding all of the [`LightSegment`](LightSegmentBundle)s
/// and despawning [`LightRaySource`]s when the level changes.
pub fn cleanup_light_sources(
    mut commands: Commands,
    q_light_sources: Query<Entity, With<LightRaySource>>,
    segment_cache: Res<LightSegmentCache>,
    mut q_segments: Query<(&mut Transform, &mut Visibility), With<LightSegmentMarker>>,
) {
    // FIXME: should make these entities children of the level so that they are despawned
    // automagically (?)

    for entity in q_light_sources.iter() {
        commands.entity(entity).despawn_recursive();
    }

    segment_cache.table.iter().for_each(|(_, items)| {
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
