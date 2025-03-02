use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::{mesh::PrimitiveTopology, view::RenderLayers},
};
use bevy_rapier2d::prelude::*;

use crate::camera::MainCamera;

use super::{
    combine_lights,
    light::{draw_lights, LineLighting, PointLighting},
    material::FrameMaskMaterial,
    CombinedLighting, LightingRenderData, FRAMES_LAYER,
};

/// [`Component`] that automatically attaches [`Occluder`] components as children of an entity based on the shape of its [`Collider`].
/// Also automatically removes and reattaches [`Occluder`]s when the [`Collider`] is removed/reattached (e.g. crystals).
/// Currently only works on cuboid colliders.
#[derive(Component)]
pub struct ColliderBasedOccluder {
    /// How many pixels to reduce the size of the collider by.
    /// Used mainly to make crystal occluders a bit smaller to allow light to pass through in a cool way.
    pub indent: f32,
}

impl Default for ColliderBasedOccluder {
    fn default() -> Self {
        Self { indent: 0.0 }
    }
}

/// [`Component`] that represents a line that prevents light from passing through.
/// The two points that make up the line are calculated by adding `point_1_offset` and `point_2_offset`
/// to the entity's [`Transform`].
#[derive(Component)]
#[require(Transform)]
pub struct Occluder {
    pub point_1_offset: Vec2,
    pub point_2_offset: Vec2,
}

pub struct OccluderPlugin;
impl Plugin for OccluderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_collider_occluders)
            .add_systems(Update, despawn_collider_occluders)
            .add_systems(Update, spawn_occluder_renderers)
            .add_observer(on_remove_occluder_despawn_occluder_renderers)
            .add_systems(PostUpdate, draw_occluders.after(draw_lights));
    }
}

/// [`Component`] used to internally to render each occluder. Every time an [`Occluder`] component
/// is spawned/inserted, 16 [`OccluderRendererBundle`]s are spawned, one to occlude each light source. They are despawned
/// when the entity is despawned or the [`Occluder`] component is removed.
///
/// Note: You shouldn't use this component, use [`Occluder`] instead.
#[derive(Component)]
struct OccluderRenderer {
    pub frame_index: usize,
    pub occluder: Entity,
}

#[derive(Bundle)]
struct OccluderRendererBundle {
    occluder_renderer: OccluderRenderer,
    render_layers: RenderLayers,
    material: MeshMaterial2d<FrameMaskMaterial>,
    mesh: Mesh2d,
    transform: Transform,
    visibility: Visibility,
}

impl OccluderRendererBundle {
    fn new(frame_index: usize, occluder: Entity, render: &Res<LightingRenderData>) -> Self {
        OccluderRendererBundle {
            occluder_renderer: OccluderRenderer {
                frame_index,
                occluder,
            },
            render_layers: FRAMES_LAYER,
            material: MeshMaterial2d(render.frame_mask_materials[frame_index].clone()),
            mesh: Mesh2d(render.default_occluder_mesh.clone()),
            transform: Transform::default(),
            visibility: Visibility::Visible,
        }
    }
}

fn spawn_collider_occluders(
    mut commands: Commands,
    q_colliders: Query<(Entity, &Collider, &ColliderBasedOccluder), Added<Collider>>,
) {
    for (entity, (point_1, point_2)) in q_colliders
        .iter()
        .map(
            |(entity, collider, collider_based_occluder)| match collider.as_typed_shape() {
                ColliderView::Cuboid(cub) => {
                    let (half_x, half_y) = cub.half_extents().into();
                    let half_x = half_x - collider_based_occluder.indent;
                    let half_y = half_y - collider_based_occluder.indent;
                    let four_corners = [(-1., -1.), (-1., 1.), (1., 1.), (1., -1.)]
                        .map(|(x, y)| Vec2::new(half_x * x, half_y * y));
                    (
                        entity,
                        [
                            (four_corners[0], four_corners[1]),
                            (four_corners[1], four_corners[2]),
                            (four_corners[2], four_corners[3]),
                            (four_corners[3], four_corners[0]),
                        ],
                    )
                }
                _ => panic!("Tried adding occluder based on non-cuboid collider"),
            },
        )
        .flat_map(|(entity, sides)| sides.into_iter().map(move |side| (entity, side)))
    {
        let occluder = commands
            .spawn((Occluder {
                point_1_offset: point_1,
                point_2_offset: point_2,
            },))
            .id();
        commands.entity(entity).add_child(occluder);
    }
}

fn despawn_collider_occluders(
    mut commands: Commands,
    mut removed: RemovedComponents<Collider>,
    q_colliders: Query<(Entity, &Children), With<ColliderBasedOccluder>>,
    q_occluders: Query<Entity, With<Occluder>>,
) {
    for removed_collider_entity in removed.read() {
        let Ok((collider_entity, children)) = q_colliders.get(removed_collider_entity) else {
            continue;
        };
        if collider_entity != removed_collider_entity {
            continue;
        }
        for child in children.iter() {
            let Ok(occluder_entity) = q_occluders.get(*child) else {
                continue;
            };
            commands.entity(occluder_entity).despawn();
        }
    }
}

fn spawn_occluder_renderers(
    mut commands: Commands,
    q_occluders: Query<Entity, Added<Occluder>>,
    render: Res<LightingRenderData>,
) {
    for entity in q_occluders.iter() {
        for frame_index in 0..16 {
            commands.spawn(OccluderRendererBundle::new(frame_index, entity, &render));
        }
    }
}

fn on_remove_occluder_despawn_occluder_renderers(
    removed: Trigger<OnRemove, Occluder>,
    mut commands: Commands,
    q_occluder_renderers: Query<(Entity, &OccluderRenderer)>,
) {
    let removed_occluder = removed.entity();
    for (entity, occluder_renderer) in q_occluder_renderers.iter() {
        if occluder_renderer.occluder == removed_occluder {
            commands.entity(entity).despawn();
        }
    }
}

fn vector_to_segment(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    // Vector from A to P
    let ap = p - a;

    // Vector from A to B
    let ab = b - a;

    // Squared length of AB
    let ab_length_squared = ab.dot(ab);

    // Handle degenerate case: A and B are the same point
    if ab_length_squared == 0.0 {
        return -ap; // The vector directly towards A (or B, since they're the same)
    }

    // Projection of P onto the infinite line defined by A and B
    let t = f32::clamp(ap.dot(ab) / ab_length_squared, 0.0, 1.0); // Clamp t to [0, 1]

    // Closest point on the segment
    let closest_point = a + t * ab;

    // Return vector towards the segment
    closest_point - p
}

#[allow(clippy::type_complexity)]
fn draw_occluders(
    mut meshes: ResMut<Assets<Mesh>>,

    mut q_occluder_renderers: Query<
        (
            &OccluderRenderer,
            &mut Visibility,
            &mut Transform,
            &mut Mesh2d,
        ),
        (
            Without<MainCamera>,
            Without<LineLighting>,
            Without<PointLighting>,
        ),
    >,

    q_occluder: Query<(&Occluder, &GlobalTransform)>,
    q_camera: Query<&Transform, With<MainCamera>>,

    q_line_lights: Query<(&GlobalTransform, &Visibility, &LineLighting)>,
    q_point_lights: Query<(&GlobalTransform, &Visibility, &PointLighting)>,
) {
    let Ok(camera_t) = q_camera.get_single() else {
        return;
    };
    let camera_translation = camera_t.translation.truncate();

    let lights = combine_lights(q_line_lights, q_point_lights, 16);

    for (occluder_renderer, mut visibility, mut transform, mut mesh) in
        q_occluder_renderers.iter_mut()
    {
        let Ok((occluder, occluder_transform)) = q_occluder.get(occluder_renderer.occluder) else {
            continue;
        };
        let Some(CombinedLighting {
            pos_1: light_pos_1,
            pos_2: light_pos_2,
            radius,
            ..
        }) = lights.get(occluder_renderer.frame_index)
        else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let point_1 = occluder.point_1_offset + occluder_transform.translation().truncate();
        let point_2 = occluder.point_2_offset + occluder_transform.translation().truncate();

        let frame_i = occluder_renderer.frame_index % 4;
        let frame_j = occluder_renderer.frame_index / 4;

        let point_1_to_segment = vector_to_segment(point_1, *light_pos_1, *light_pos_2);
        let point_2_to_segment = vector_to_segment(point_2, *light_pos_1, *light_pos_2);

        let camera_point = |p: Vec2| p - camera_translation + Vec2::new(320.0 * 0.5, -180. * 0.5);

        let out_of_bounds = |p: Vec2| {
            let x = p.x;
            let y = p.y;
            let buffer = 4.0;
            !(x > -buffer && x < 320. + buffer && y < buffer && y > -180.0 - buffer)
        };

        let frame_point = |p: Vec2| {
            camera_point(p)
                + Vec2::new(320.0 * -2., 180. * 2.)
                + Vec2::new(320. * frame_i as f32, -180. * frame_j as f32)
        };

        let point_1_frame = frame_point(point_1);

        let pos = point_1_frame;

        let is_out_of_bounds =
            out_of_bounds(camera_point(point_1)) || out_of_bounds(camera_point(point_2));

        let is_out_of_lights = point_1_to_segment.length() > *radius
            && point_2_to_segment.length() > *radius
            && (point_1 - point_2).length() < *radius * 2.0;

        if is_out_of_bounds || is_out_of_lights {
            *visibility = Visibility::Hidden;
            continue;
        }

        let occluder_polygon =
            create_occluder_polygon(point_1, point_2, point_1_to_segment, point_2_to_segment);

        let shape = meshes.add(polygon_mesh(&occluder_polygon.map(|x| x - point_1)));
        *transform = Transform::default().with_translation(pos.extend(1.0));
        *mesh = Mesh2d(shape);
        *visibility = Visibility::Visible;
    }
}

fn create_occluder_polygon(
    point_1: Vec2,
    point_2: Vec2,
    point_1_to_segment: Vec2,
    point_2_to_segment: Vec2,
) -> [Vec2; 4] {
    [
        point_1,
        point_2,
        point_2 - point_2_to_segment.normalize() * 1000.0,
        point_1 - point_1_to_segment.normalize() * 1000.0,
    ]
}
fn polygon_mesh(vertices: &[Vec2]) -> Mesh {
    let mut triangles = Vec::new();
    for i in 0..(vertices.len() - 1) {
        triangles.extend(
            [
                vertices[0].extend(0.),
                vertices[i].extend(0.),
                vertices[i + 1].extend(0.),
            ]
            .map(|v| [v.x, v.y, v.z]),
        );
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; triangles.len()])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; triangles.len()])
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, triangles)
}
