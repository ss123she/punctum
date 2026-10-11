use std::sync::Arc;

use winit::window::Window;

use crate::{
    gpu::GpuContext, pipeline::RenderPipeline, punctum::RenderConfig, window::RenderWindow,
};

pub struct State {
    gpu: GpuContext,
    render_window: RenderWindow,
    pipeline: RenderPipeline,
}

impl State {
    pub(crate) async fn new(window: Arc<Window>, config: RenderConfig) -> anyhow::Result<Self> {
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
        let render_window = RenderWindow::new(window, &gpu, surface, &config)?;

        let pipeline = RenderPipeline::new(&gpu, render_window.surface_config(), &config)?;

        Ok(Self {
            gpu,
            render_window,
            pipeline,
        })
    }

    pub(crate) fn update(&mut self) {}

    pub(crate) fn render(&mut self) -> anyhow::Result<()> {
        self.render_window.window().request_redraw();

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
                label: Some("Render Encoder"),
            });

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

            render_pass.set_pipeline(&self.pipeline.render_pipeline); // 2.
            render_pass.draw(0..3, 0..1); // 3.
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        self.gpu.queue.present(output);

        Ok(())
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.render_window.resize(&self.gpu, width, height);
    }
}
