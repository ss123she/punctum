use std::sync::Arc;

use wgpu::{TextureDescriptor, TextureUsages};
use winit::window::Window;

use crate::{
    ComputeConfig, RenderConfig,
    core::{GpuContext, RenderWindow},
    pipeline::{ComputePipeline, RenderPipeline},
};

pub(crate) struct State {
    gpu: GpuContext,
    render_window: RenderWindow,
    compute_pipeline: ComputePipeline,
    render_pipeline: RenderPipeline,
    sampler: wgpu::Sampler,
    target_resources: TargetResources,
}

pub(crate) struct TargetResources {
    pub(crate) texture: wgpu::Texture,
    pub(crate) compute_bind_group: wgpu::BindGroup,
    pub(crate) render_bind_group: wgpu::BindGroup,
}

impl TargetResources {
    pub(crate) fn new(
        gpu: &GpuContext,
        compute_pipeline: &ComputePipeline,
        render_pipeline: &RenderPipeline,
        sampler: &wgpu::Sampler,
        width: u32,
        height: u32,
    ) -> Self {
        let texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Target texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let compute_bind_group = compute_pipeline.create_bind_group(gpu, &view);
        let render_bind_group = render_pipeline.create_bind_group(gpu, &view, sampler);

        Self {
            texture,
            compute_bind_group,
            render_bind_group,
        }
    }
}

impl State {
    pub(crate) async fn new(
        window: Arc<Window>,
        compute_config: &ComputeConfig,
        render_config: &RenderConfig,
    ) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::BROWSER_WEBGPU,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone()).unwrap();
        let gpu = GpuContext::new(&instance, Some(&surface)).await?;
        let render_window = RenderWindow::new(window, &gpu, surface, render_config)?;

        let compute_pipeline = ComputePipeline::new(&gpu, compute_config)?;
        let render_pipeline =
            RenderPipeline::new(&gpu, render_window.surface_config(), render_config)?;

        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Blit Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let surface_config = render_window.surface_config();
        let target_resources = TargetResources::new(
            &gpu,
            &compute_pipeline,
            &render_pipeline,
            &sampler,
            surface_config.width,
            surface_config.height,
        );

        Ok(Self {
            gpu,
            render_window,
            render_pipeline,
            compute_pipeline,
            sampler,
            target_resources,
        })
    }

    pub(crate) fn update(&mut self) {}

    pub(crate) fn render(&mut self) -> anyhow::Result<()> {
        let Some(output) = self.render_window.get_current_texture(&self.gpu)? else {
            return Ok(());
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Frame Encoder"),
            });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            self.compute_pipeline.dispatch(
                &mut cpass,
                &self.target_resources.compute_bind_group,
                16, // TODO: UNHARDCODE
                16, // TODO: UNHARDCODE
                1,  // TODO: UNHARDCODE
            );
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            render_pass.set_pipeline(&self.render_pipeline.render_pipeline);
            render_pass.set_bind_group(0, &self.target_resources.render_bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        self.gpu.queue.present(output);

        Ok(())
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.render_window.resize(&self.gpu, width, height);

        if width > 0 && height > 0 {
            self.target_resources = TargetResources::new(
                &self.gpu,
                &self.compute_pipeline,
                &self.render_pipeline,
                &self.sampler,
                width,
                height,
            );
        }
    }

    pub(crate) fn window(&self) -> &Window {
        self.render_window.window()
    }
}
