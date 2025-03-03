use std::ops::Range;

use bevy::{
    ecs::{
        entity::EntityHashSet,
        query::{QueryItem, ROQueryItem},
        system::SystemParamItem,
    },
    math::FloatOrd,
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
        render_phase::{
            CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions, PhaseItem,
            PhaseItemExtraIndex, RenderCommand, RenderCommandResult, SetItemPipeline,
            SortedPhaseItem, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{
            binding_types::{sampler, texture_2d},
            *,
        },
        renderer::{RenderContext, RenderDevice},
        sync_world::{MainEntity, RenderEntity},
        view::{RenderVisibleEntities, ViewTarget},
        Extract,
    },
    sprite::SetMesh2dViewBindGroup,
};

use super::{
    ambient_light::{AmbientLight2dPipeline, SetAmbientLight2dBindGroup},
    line_light::{
        DrawLineLight2d, ExtractLineLight2d, LineLight2dBounds, LineLight2dPipeline,
        SetLineLight2dBindGroup,
    },
    occluder::{
        DrawOccluder2d, ExtractOccluder2d, Occluder2dBounds, Occluder2dPipeline,
        OccluderCountTexture, SetOccluder2dBindGroup,
    },
    AmbientLight2d, LineLight2d, Occluder2d,
};

/// Deferred Lighting [`SortedPhaseItem`]s.
pub struct DeferredLighting2d {
    /// The key, which determines which can be batched.
    pub sort_key: FloatOrd,
    /// An entity from which data will be fetched, including the mesh if
    /// applicable.
    pub entity: (Entity, MainEntity),
    /// The identifier of the render pipeline.
    pub pipeline: CachedRenderPipelineId,
    /// The function used to draw.
    pub draw_function: DrawFunctionId,
    /// The ranges of instances.
    pub batch_range: Range<u32>,
    /// An extra index, which is either a dynamic offset or an index in the
    /// indirect parameters list.
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for DeferredLighting2d {
    #[inline]
    fn entity(&self) -> Entity {
        self.entity.0
    }
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
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index
    }
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl SortedPhaseItem for DeferredLighting2d {
    type SortKey = FloatOrd;

    fn sort_key(&self) -> Self::SortKey {
        self.sort_key
    }
}

impl CachedRenderPipelinePhaseItem for DeferredLighting2d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

#[derive(Resource)]
pub struct PostProcessRes {
    sampler: Sampler,
    pub layout: BindGroupLayout,
}

impl FromWorld for PostProcessRes {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(
            "post_process_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::NonFiltering),
                ),
            ),
        );
        let sampler = render_device.create_sampler(&SamplerDescriptor::default());
        Self { sampler, layout }
    }
}

pub fn extract_deferred_lighting_2d_camera_phases(
    mut phases: ResMut<ViewSortedRenderPhases<DeferredLighting2d>>,
    cameras_2d: Extract<Query<(RenderEntity, &Camera), With<Camera2d>>>,
    mut live_entities: Local<EntityHashSet>,
) {
    live_entities.clear();
    for (entity, camera) in &cameras_2d {
        if !camera.is_active {
            continue;
        }
        phases.insert_or_clear(entity);
        live_entities.insert(entity);
    }
    // Clear out all dead views.
    phases.retain(|camera_entity, _| live_entities.contains(camera_entity));
}

#[allow(clippy::too_many_arguments)]
pub fn queue_deferred_lighting(
    deferred_lighting_draw_functions: Res<DrawFunctions<DeferredLighting2d>>,
    occluder_pipeline: Res<Occluder2dPipeline>,
    line_light_pipeline: Res<LineLight2dPipeline>,
    ambient_light_pipeline: Res<AmbientLight2dPipeline>,
    q_line_lights: Query<&LineLight2dBounds, With<ExtractLineLight2d>>,
    q_occluder: Query<&Occluder2dBounds, With<ExtractOccluder2d>>,
    mut deferred_lighting_phases: ResMut<ViewSortedRenderPhases<DeferredLighting2d>>,
    views: Query<(Entity, &MainEntity, &RenderVisibleEntities), With<AmbientLight2d>>,
) {
    // TODO: ignore invisible entities

    for (view_e, view_me, visible_entities) in views.iter() {
        let Some(phase) = deferred_lighting_phases.get_mut(&view_e) else {
            continue;
        };

        let prepare_deferred_lighting = deferred_lighting_draw_functions
            .read()
            .id::<PrepareDeferredLighting>();
        let render_ambient_light = deferred_lighting_draw_functions
            .read()
            .id::<RenderAmbientLight2d>();
        let render_occluder = deferred_lighting_draw_functions
            .read()
            .id::<RenderOccluder>();
        let prepare_line_light = deferred_lighting_draw_functions
            .read()
            .id::<PrepareLineLight2d>();
        let render_line_light = deferred_lighting_draw_functions
            .read()
            .id::<RenderLineLight2d>();
        let reset_stencil_buffer = deferred_lighting_draw_functions
            .read()
            .id::<ResetOccluderStencil>();

        let mut sort_key = 0.0;

        let mut add_phase_item = |pipeline: CachedRenderPipelineId,
                                  draw_function: DrawFunctionId,
                                  entity: (Entity, MainEntity)| {
            phase.add(DeferredLighting2d {
                pipeline,
                draw_function,
                entity,
                batch_range: 0..1,
                sort_key: FloatOrd(sort_key),
                extra_index: PhaseItemExtraIndex::NONE,
            });
            sort_key += 1.0;
        };

        // Set bind group 0 - post process uniform
        // Set bind group 1 - view uniform
        add_phase_item(
            ambient_light_pipeline.pipeline_id,
            prepare_deferred_lighting,
            (view_e, *view_me),
        );

        // Draw ambient light
        add_phase_item(
            ambient_light_pipeline.pipeline_id,
            render_ambient_light,
            (view_e, *view_me),
        );

        // Start rendering lights
        for (pl_e, pl_me) in visible_entities.iter::<With<LineLight2d>>() {
            let Ok(light_bounds) = q_line_lights.get(*pl_e) else {
                continue;
            };
            // Set bind group 2 - line light uniform
            add_phase_item(
                line_light_pipeline.pipeline_id,
                prepare_line_light,
                (*pl_e, *pl_me),
            );

            // Render occluder shadows
            for (ocl_e, ocl_me) in visible_entities.iter::<With<Occluder2d>>() {
                let Ok(occluder_bounds) = q_occluder.get(*ocl_e) else {
                    continue;
                };
                if !occluder_bounds.visible_from_line_light(light_bounds) {
                    continue;
                }
                add_phase_item(
                    occluder_pipeline.shadow_pipeline_id,
                    render_occluder,
                    (*ocl_e, *ocl_me),
                );
            }

            // Cutout occluder bodies
            for (ocl_e, ocl_me) in visible_entities.iter::<With<Occluder2d>>() {
                let Ok(occluder_bounds) = q_occluder.get(*ocl_e) else {
                    continue;
                };
                if !occluder_bounds.visible_from_line_light(light_bounds) {
                    continue;
                }
                add_phase_item(
                    occluder_pipeline.cutout_pipeline_id,
                    render_occluder,
                    (*ocl_e, *ocl_me),
                );
            }

            // Render the actual light now
            add_phase_item(
                line_light_pipeline.pipeline_id,
                render_line_light,
                (*pl_e, *pl_me),
            );

            // Reset the occluder
            add_phase_item(
                occluder_pipeline.reset_pipeline_id,
                reset_stencil_buffer,
                (*pl_e, *pl_me),
            );
        }
    }
}

pub type PrepareDeferredLighting = (
    // SetPostProcessBindGroup<0>,
    SetMesh2dViewBindGroup<1>,
);

pub type RenderAmbientLight2d = (SetItemPipeline, SetAmbientLight2dBindGroup<2>, DrawTriangle);

pub type PrepareLineLight2d = SetLineLight2dBindGroup<2>;

pub type RenderOccluder = (
    SetItemPipeline,
    // SetLineLight2dBindGroup<2>,
    SetOccluder2dBindGroup<3>,
    DrawOccluder2d,
);

pub type RenderLineLight2d = (SetItemPipeline, SetLineLight2dBindGroup<2>, DrawLineLight2d);

pub type ResetOccluderStencil = (SetItemPipeline, DrawTriangle);

pub struct DrawTriangle;
impl<P: PhaseItem> RenderCommand<P> for DrawTriangle {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.draw(0..3, 0..1);

        RenderCommandResult::Success
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct DeferredLightingLabel;

#[derive(Default)]
pub struct DeferredLightingNode;

impl ViewNode for DeferredLightingNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static OccluderCountTexture,
        &'static AmbientLight2d,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view_target, occluder_count_texture, _ambient_lighting): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let lighting_phases = world.resource::<ViewSortedRenderPhases<DeferredLighting2d>>();
        let view_entity = graph.view_entity();
        let Some(lighting_phase) = lighting_phases.get(&view_entity) else {
            return Ok(());
        };

        let post_process_res = world.resource::<PostProcessRes>();
        let post_process = view_target.post_process_write();
        let post_process_group = render_context.render_device().create_bind_group(
            "post_process_group",
            &post_process_res.layout,
            &BindGroupEntries::sequential((post_process.source, &post_process_res.sampler)),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("deferred_lighting_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: occluder_count_texture.0.view(),
                depth_ops: None,
                stencil_ops: Some(Operations {
                    load: LoadOp::Clear(0),
                    store: StoreOp::Discard,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_bind_group(0, &post_process_group, &[]);

        if !lighting_phase.items.is_empty() {
            if let Err(err) = lighting_phase.render(&mut render_pass, world, view_entity) {
                error!("Error encountered while rendering the 2d deferred lighting phase {err:?}")
            }
        }

        Ok(())
    }
}
