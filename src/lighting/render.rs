use std::ops::Range;

use bevy::{
    ecs::{
        entity::EntityHashSet,
        query::{QueryItem, ROQueryItem},
        system::SystemParamItem,
    },
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
        render_phase::{
            BinnedPhaseItem, BinnedRenderPhaseType, CachedRenderPipelinePhaseItem, DrawFunctionId,
            DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand, RenderCommandResult,
            SetItemPipeline, TrackedRenderPass, ViewBinnedRenderPhases,
        },
        render_resource::{
            binding_types::{sampler, texture_2d},
            *,
        },
        renderer::{RenderContext, RenderDevice},
        sync_world::{MainEntity, RenderEntity},
        view::ViewTarget,
        Extract,
    },
    sprite::SetMesh2dViewBindGroup,
};

use super::{
    ambient_light::{AmbientLight2dPipeline, SetAmbientLight2dBindGroup},
    occluder::{
        DrawOccluder2d, ExtractOccluder2d, Occluder2dBounds, Occluder2dPipeline,
        OccluderCountTexture, SetOccluder2dBindGroup,
    },
    point_light::{
        DrawPointLight2d, ExtractPointLight2d, PointLight2dBounds, PointLight2dPipeline,
        SetPointLight2dBindGroup,
    },
    AmbientLight2d,
};

/// Deferred Lighting [`BinnedPhaseItem`]s.
pub struct DeferredLighting2d {
    /// The key, which determines which can be batched.
    pub key: DeferredLightingBinKey,
    /// An entity from which data will be fetched, including the mesh if
    /// applicable.
    pub representative_entity: (Entity, MainEntity),
    /// The ranges of instances.
    pub batch_range: Range<u32>,
    /// An extra index, which is either a dynamic offset or an index in the
    /// indirect parameters list.
    pub extra_index: PhaseItemExtraIndex,
}

/// Data that must be identical in order to batch phase items together.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeferredLightingBinKey {
    /// The identifier of the render pipeline.
    pub pipeline: CachedRenderPipelineId,
    /// The function used to draw.
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for DeferredLighting2d {
    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity.0
    }
    fn main_entity(&self) -> MainEntity {
        self.representative_entity.1
    }
    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.key.draw_function
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

impl BinnedPhaseItem for DeferredLighting2d {
    type BinKey = DeferredLightingBinKey;

    fn new(
        key: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        DeferredLighting2d {
            key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for DeferredLighting2d {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.key.pipeline
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
    mut phases: ResMut<ViewBinnedRenderPhases<DeferredLighting2d>>,
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

pub fn queue_deferred_lighting(
    deferred_lighting_draw_functions: Res<DrawFunctions<DeferredLighting2d>>,
    occluder_pipeline: Res<Occluder2dPipeline>,
    point_light_pipeline: Res<PointLight2dPipeline>,
    ambient_light_pipeline: Res<AmbientLight2dPipeline>,
    q_point_lights: Query<(Entity, &MainEntity, &PointLight2dBounds), With<ExtractPointLight2d>>,
    q_occluder: Query<(Entity, &MainEntity, &Occluder2dBounds), With<ExtractOccluder2d>>,
    mut deferred_lighting_phases: ResMut<ViewBinnedRenderPhases<DeferredLighting2d>>,
    views: Query<(Entity, &MainEntity), With<AmbientLight2d>>,
) {
    for (view_e, view_me) in views.iter() {
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
        let prepare_point_light = deferred_lighting_draw_functions
            .read()
            .id::<PreparePointLight2d>();
        let render_point_light = deferred_lighting_draw_functions
            .read()
            .id::<RenderPointLight2d>();
        let reset_stencil_buffer = deferred_lighting_draw_functions
            .read()
            .id::<ResetOccluderStencil>();

        // Set bind group 0 - post process uniform
        // Set bind group 1 - view uniform
        phase.add(
            DeferredLightingBinKey {
                pipeline: ambient_light_pipeline.pipeline_id,
                draw_function: prepare_deferred_lighting,
            },
            (view_e, *view_me),
            BinnedRenderPhaseType::NonMesh,
        );

        // Render ambient light into the scene
        phase.add(
            DeferredLightingBinKey {
                pipeline: ambient_light_pipeline.pipeline_id,
                draw_function: render_ambient_light,
            },
            (view_e, *view_me),
            BinnedRenderPhaseType::NonMesh,
        );

        // Start rendering lights
        for (pl_e, pl_me, light_bounds) in q_point_lights.iter() {
            // Set bind group 2 - point light uniform
            phase.add(
                DeferredLightingBinKey {
                    pipeline: point_light_pipeline.pipeline_id,
                    draw_function: prepare_point_light,
                },
                (pl_e, *pl_me),
                BinnedRenderPhaseType::NonMesh,
            );

            // Render occluder shadows
            for (ocl_e, ocl_me, occluder_bounds) in q_occluder.iter() {
                if !occluder_bounds.visible_from_point_light(light_bounds) {
                    continue;
                }
                phase.add(
                    DeferredLightingBinKey {
                        pipeline: occluder_pipeline.shadow_pipeline_id,
                        draw_function: render_occluder,
                    },
                    (ocl_e, *ocl_me),
                    BinnedRenderPhaseType::NonMesh,
                );
            }

            // Cutout occluder bodies
            for (ocl_e, ocl_me, occluder_bounds) in q_occluder.iter() {
                if !occluder_bounds.visible_from_point_light(light_bounds) {
                    continue;
                }
                phase.add(
                    DeferredLightingBinKey {
                        pipeline: occluder_pipeline.cutout_pipeline_id,
                        draw_function: render_occluder,
                    },
                    (ocl_e, *ocl_me),
                    BinnedRenderPhaseType::NonMesh,
                );
            }

            // Render the actual light now
            phase.add(
                DeferredLightingBinKey {
                    pipeline: point_light_pipeline.pipeline_id,
                    draw_function: render_point_light,
                },
                (pl_e, *pl_me),
                BinnedRenderPhaseType::NonMesh,
            );

            // Reset the occluder
            phase.add(
                DeferredLightingBinKey {
                    pipeline: occluder_pipeline.reset_pipeline_id,
                    draw_function: reset_stencil_buffer,
                },
                (pl_e, *pl_me),
                BinnedRenderPhaseType::NonMesh,
            );
        }
    }
}

pub type PrepareDeferredLighting = (
    // SetPostProcessBindGroup<0>,
    SetMesh2dViewBindGroup<1>,
);

pub type RenderAmbientLight2d = (SetItemPipeline, SetAmbientLight2dBindGroup<2>, DrawTriangle);

pub type PreparePointLight2d = SetPointLight2dBindGroup<2>;

pub type RenderOccluder = (
    SetItemPipeline,
    // SetPointLight2dBindGroup<2>,
    SetOccluder2dBindGroup<3>,
    DrawOccluder2d,
);

pub type RenderPointLight2d = (
    SetItemPipeline,
    SetPointLight2dBindGroup<2>,
    DrawPointLight2d,
);

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
        let lighting_phases = world.resource::<ViewBinnedRenderPhases<DeferredLighting2d>>();
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
                view: &post_process.destination,
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

        if !lighting_phase.is_empty() {
            if let Err(err) = lighting_phase.render(&mut render_pass, world, view_entity) {
                error!("Error encountered while rendering the 2d deferred lighting phase {err:?}")
            }
        }

        // render_pass.set_bind_group(
        //     0,
        //     &mesh2d_view_bind_group.value,
        //     &[view_uniform_offset.offset],
        // );
        //
        // for (light_index, light_bounds) in self.point_lights.iter() {
        //     // render occluders
        //     render_pass.set_bind_group(1, &point_light_bind_group, &[*light_index]);
        //     render_pass.set_vertex_buffer(0, occluder_buffers.vertices.buffer().unwrap().slice(..));
        //     render_pass.set_index_buffer(
        //         occluder_buffers.indices.buffer().unwrap().slice(..),
        //         0,
        //         IndexFormat::Uint32,
        //     );
        //
        //     // TODO: instaced/batched rendering
        //
        //     // render_pass.set_render_pipeline(occluder_shadow_pipeline);
        //
        //     // render occluders for this light into the stencil buffer
        //     render_pass.set_render_pipeline(occluder_shadow_pipeline);
        //     for (occluder_index, occluder_bounds) in self.occluders.iter() {
        //         if !occluder_bounds.visible_from_point_light(light_bounds) {
        //             continue;
        //         }
        //         render_pass.set_bind_group(2, &occluder_bind_group, &[*occluder_index]);
        //         render_pass.draw_indexed(0..18, 0, 0..1);
        //     }
        //     // cut out all occluders for this light
        //     render_pass.set_render_pipeline(occluder_cutout_pipeline);
        //     for (occluder_index, occluder_bounds) in self.occluders.iter() {
        //         if !occluder_bounds.visible_from_point_light(light_bounds) {
        //             continue;
        //         }
        //         render_pass.set_bind_group(2, &occluder_bind_group, &[*occluder_index]);
        //         render_pass.draw_indexed(0..18, 0, 0..1);
        //     }
        //
        //     // render the light itself
        //     render_pass.set_render_pipeline(point_light_pipeline);
        //
        //     render_pass.set_bind_group(2, &point_light_frag_group, &[]);
        //
        //     render_pass.set_stencil_reference(0); // only render if no occluders here
        //
        //     render_pass
        //         .set_vertex_buffer(0, point_light_buffers.vertices.buffer().unwrap().slice(..));
        //     render_pass.set_index_buffer(
        //         point_light_buffers.indices.buffer().unwrap().slice(..),
        //         0,
        //         IndexFormat::Uint32,
        //     );
        //     render_pass.draw_indexed(0..6, 0, 0..1);
        //
        //     // reset the stencil buffer for the next light
        //     render_pass.set_render_pipeline(occluder_reset_pipeline);
        //     render_pass.draw(0..3, 0..1);
        // }

        Ok(())
    }
}
