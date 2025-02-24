use std::ops::Range;

use bevy::{
    ecs::{
        entity::EntityHashSet,
        query::ROQueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    math::{vec3, FloatOrd},
    prelude::*,
    render::{
        render_phase::{
            CachedRenderPipelinePhaseItem, DrawFunctionId, PhaseItem, PhaseItemExtraIndex,
            RenderCommand, RenderCommandResult, SetItemPipeline, SortedPhaseItem,
            TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{BufferUsages, CachedRenderPipelineId, IndexFormat, RawBufferVec},
        renderer::{RenderDevice, RenderQueue},
        sync_world::{MainEntity, RenderEntity},
        Extract,
    },
};
use bytemuck::{Pod, Zeroable};

pub struct DrawDeferredLighting2d;

impl<P> RenderCommand<P> for DrawDeferredLighting2d
where
    P: PhaseItem,
{
    type Param = SRes<DeferredLighting2dBuffers>;
    type ViewQuery = ();
    type ItemQuery = ();

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewQuery>,
        _: Option<ROQueryItem<'w, Self::ItemQuery>>,
        deferred_lighting_2d_buffers: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let buffers = deferred_lighting_2d_buffers.into_inner();

        pass.set_vertex_buffer(0, buffers.vertices.buffer().unwrap().slice(..));

        pass.set_index_buffer(
            buffers.indices.buffer().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );

        pass.draw_indexed(0..6, 0, 0..1);

        RenderCommandResult::Success
    }
}

pub type DrawDeferredLighting2dCommands = (SetItemPipeline, DrawDeferredLighting2d);

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct DeferredLighting2dVertex {
    position: Vec3,
    _pad: u32,
}

impl DeferredLighting2dVertex {
    const fn new(position: Vec3) -> Self {
        DeferredLighting2dVertex { position, _pad: 0 }
    }
}

#[derive(Resource)]
pub struct DeferredLighting2dBuffers {
    vertices: RawBufferVec<DeferredLighting2dVertex>,
    indices: RawBufferVec<u32>,
}

static VERTICES: [DeferredLighting2dVertex; 4] = [
    DeferredLighting2dVertex::new(vec3(-1.0, -1.0, 0.0)),
    DeferredLighting2dVertex::new(vec3(1.0, -1.0, 0.0)),
    DeferredLighting2dVertex::new(vec3(1.0, 1.0, 0.0)),
    DeferredLighting2dVertex::new(vec3(-1.0, 1.0, 0.0)),
];

static INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];

impl FromWorld for DeferredLighting2dBuffers {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        let mut vbo = RawBufferVec::new(BufferUsages::VERTEX);
        let mut ibo = RawBufferVec::new(BufferUsages::INDEX);

        for vtx in &VERTICES {
            vbo.push(*vtx);
        }
        for index in &INDICES {
            ibo.push(*index);
        }

        vbo.write_buffer(render_device, render_queue);
        ibo.write_buffer(render_device, render_queue);

        DeferredLighting2dBuffers {
            vertices: vbo,
            indices: ibo,
        }
    }
}

/// Deferred Lighting 2D [`SortedPhaseItem`]s.
pub struct DeferredLighting2d {
    pub sort_key: FloatOrd,
    pub entity: (Entity, MainEntity),
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for DeferredLighting2d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity.0
    }

    #[inline]
    fn main_entity(&self) -> MainEntity {
        self.entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for DeferredLighting2d {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        radsort::sort_by_key(items, |item| item.sort_key().0);
    }
}

impl CachedRenderPipelinePhaseItem for DeferredLighting2d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub fn extract_deferred_lighting_camera_phases(
    mut deferred_lighting_2d_phases: ResMut<ViewSortedRenderPhases<DeferredLighting2d>>,
    cameras_2d: Extract<Query<(RenderEntity, &Camera), With<Camera2d>>>,
    mut live_entities: Local<EntityHashSet>,
) {
    // NOTE: implement only for main camera or all cameras?

    live_entities.clear();

    for (entity, camera) in &cameras_2d {
        if !camera.is_active {
            continue;
        }
        deferred_lighting_2d_phases.insert_or_clear(entity);
        live_entities.insert(entity);
    }

    deferred_lighting_2d_phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
}
