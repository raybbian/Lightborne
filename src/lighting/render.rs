use bevy::{
    ecs::{query::QueryItem, system::SystemState},
    prelude::*,
    render::{
        extract_component::{ComponentUniforms, DynamicUniformIndex},
        render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
        render_resource::*,
        renderer::RenderContext,
        view::{ViewTarget, ViewUniformOffset},
    },
    sprite::Mesh2dViewBindGroup,
};

use super::{
    ambient_light::AmbientLight2dPipeline,
    occluder::{
        OccluderBounds, OccluderBuffers, OccluderCountTexture, OccluderPipeline, RenderOccluder,
    },
    point_light::{
        PointLight2dBounds, PointLight2dBuffers, PointLight2dPipeline, RenderPointLight2d,
    },
    AmbientLight2d,
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct DeferredLightingLabel;

#[derive(Default)]
pub struct DeferredLightingNode {
    point_lights: Vec<(u32, PointLight2dBounds)>,
    occluders: Vec<(u32, OccluderBounds)>,
}

impl ViewNode for DeferredLightingNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static OccluderCountTexture,
        &'static ViewUniformOffset,
        &'static Mesh2dViewBindGroup,
        &'static DynamicUniformIndex<AmbientLight2d>,
    );

    fn update(&mut self, world: &mut World) {
        let mut state = SystemState::<(
            Query<(
                &DynamicUniformIndex<RenderPointLight2d>,
                &PointLight2dBounds,
            )>,
            Query<(&DynamicUniformIndex<RenderOccluder>, &OccluderBounds)>,
        )>::new(world);

        let (q_indices, q_occluders) = state.get(world);

        // should this be done every frame?
        self.point_lights.clear();
        for (light_index, light_bounds) in q_indices.iter() {
            self.point_lights.push((light_index.index(), *light_bounds));
        }
        self.occluders.clear();
        for (occluder_index, occluder_bounds) in q_occluders.iter() {
            self.occluders
                .push((occluder_index.index(), *occluder_bounds));
        }
    }

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            view_target,
            occluder_count_texture,
            view_uniform_offset,
            mesh2d_view_bind_group,
            ambient_lighting_index,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let point_light_pipeline_res = world.resource::<PointLight2dPipeline>();
        let ambient_light_pipeline_res = world.resource::<AmbientLight2dPipeline>();
        let occluder_pipeline_res = world.resource::<OccluderPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let (
            Some(point_light_pipeline),
            Some(ambient_light_pipeline),
            Some(occluder_shadow_pipeline),
            Some(occluder_cutout_pipeline),
            Some(occluder_reset_pipeline),
        ) = (
            pipeline_cache.get_render_pipeline(point_light_pipeline_res.pipeline_id),
            pipeline_cache.get_render_pipeline(ambient_light_pipeline_res.pipeline_id),
            pipeline_cache.get_render_pipeline(occluder_pipeline_res.shadow_pipeline_id),
            pipeline_cache.get_render_pipeline(occluder_pipeline_res.cutout_pipeline_id),
            pipeline_cache.get_render_pipeline(occluder_pipeline_res.reset_pipeline_id),
        )
        else {
            return Ok(());
        };

        let Some(point_light_buffers) = world.get_resource::<PointLight2dBuffers>() else {
            return Ok(());
        };

        let Some(occluder_buffers) = world.get_resource::<OccluderBuffers>() else {
            return Ok(());
        };

        let point_light_2d_uniforms = world.resource::<ComponentUniforms<RenderPointLight2d>>();
        let Some(point_light_uniforms_binding) = point_light_2d_uniforms.uniforms().binding()
        else {
            return Ok(());
        };

        let ambient_light_2d_uniforms = world.resource::<ComponentUniforms<AmbientLight2d>>();
        let Some(ambient_light_2d_uniforms_binding) =
            ambient_light_2d_uniforms.uniforms().binding()
        else {
            return Ok(());
        };

        let occluder_uniforms = world.resource::<ComponentUniforms<RenderOccluder>>();
        let Some(occluder_uniforms_binding) = occluder_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        let point_light_bind_group = render_context.render_device().create_bind_group(
            "point_light_bind_group",
            &point_light_pipeline_res.bind_layout,
            &BindGroupEntries::single(point_light_uniforms_binding.clone()),
        );

        let point_light_frag_group = render_context.render_device().create_bind_group(
            "point_light_frag_group",
            &point_light_pipeline_res.frag_layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &point_light_pipeline_res.scene_sampler,
            )),
        );

        let ambient_light_frag_group = render_context.render_device().create_bind_group(
            "ambient_light_frag_group",
            &ambient_light_pipeline_res.layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &ambient_light_pipeline_res.scene_sampler,
                ambient_light_2d_uniforms_binding.clone(),
            )),
        );

        let occluder_bind_group = render_context.render_device().create_bind_group(
            "occluder_bind_group",
            &occluder_pipeline_res.layout,
            &BindGroupEntries::single(occluder_uniforms_binding.clone()),
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

        // Draw the ambient light first
        render_pass.set_render_pipeline(ambient_light_pipeline);
        render_pass.set_bind_group(
            0,
            &ambient_light_frag_group,
            &[ambient_lighting_index.index()],
        );
        render_pass.draw(0..3, 0..1);

        render_pass.set_bind_group(
            0,
            &mesh2d_view_bind_group.value,
            &[view_uniform_offset.offset],
        );

        for (light_index, light_bounds) in self.point_lights.iter() {
            // render occluders
            render_pass.set_bind_group(1, &point_light_bind_group, &[*light_index]);
            render_pass.set_vertex_buffer(0, occluder_buffers.vertices.buffer().unwrap().slice(..));
            render_pass.set_index_buffer(
                occluder_buffers.indices.buffer().unwrap().slice(..),
                0,
                IndexFormat::Uint32,
            );

            // TODO: instaced/batched rendering

            // render_pass.set_render_pipeline(occluder_shadow_pipeline);

            // render occluders for this light into the stencil buffer
            render_pass.set_render_pipeline(occluder_shadow_pipeline);
            for (occluder_index, occluder_bounds) in self.occluders.iter() {
                if !occluder_bounds.visible_from_point_light(light_bounds) {
                    continue;
                }
                render_pass.set_bind_group(2, &occluder_bind_group, &[*occluder_index]);
                render_pass.draw_indexed(0..18, 0, 0..1);
            }
            // cut out all occluders for this light
            render_pass.set_render_pipeline(occluder_cutout_pipeline);
            for (occluder_index, occluder_bounds) in self.occluders.iter() {
                if !occluder_bounds.visible_from_point_light(light_bounds) {
                    continue;
                }
                render_pass.set_bind_group(2, &occluder_bind_group, &[*occluder_index]);
                render_pass.draw_indexed(0..18, 0, 0..1);
            }

            // render the light itself
            render_pass.set_render_pipeline(point_light_pipeline);

            render_pass.set_bind_group(2, &point_light_frag_group, &[]);

            render_pass.set_stencil_reference(0); // only render if no occluders here

            render_pass
                .set_vertex_buffer(0, point_light_buffers.vertices.buffer().unwrap().slice(..));
            render_pass.set_index_buffer(
                point_light_buffers.indices.buffer().unwrap().slice(..),
                0,
                IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..6, 0, 0..1);

            // reset the stencil buffer for the next light
            render_pass.set_render_pipeline(occluder_reset_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}
