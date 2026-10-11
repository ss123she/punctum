use std::sync::Arc;

use wgpu::{Surface, SurfaceConfiguration, SurfaceTexture};
use winit::window::Window;

use crate::{gpu::GpuContext, punctum::RenderConfig};

pub(crate) struct RenderWindow {
    window: Arc<Window>,
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    is_surface_configured: bool,
}

impl RenderWindow {
    pub(crate) fn new(
        window: Arc<Window>,
        gpu: &GpuContext,
        surface: Surface<'static>,
        config: &RenderConfig,
    ) -> anyhow::Result<RenderWindow> {
        let size = window.inner_size();

        let surface_caps = surface.get_capabilities(&gpu.adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: config.present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };

        surface.configure(&gpu.device, &surface_config);
        Ok(Self {
            window,
            surface,
            surface_config,
            is_surface_configured: true,
        })
    }

    pub(crate) fn resize(&mut self, gpu: &GpuContext, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&gpu.device, &self.surface_config);
            self.is_surface_configured = true;
        }
    }

    pub(crate) fn get_current_texture(
        &mut self,
        gpu: &GpuContext,
    ) -> anyhow::Result<Option<SurfaceTexture>> {
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&gpu.device, &self.surface_config);
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                anyhow::bail!("Lost device");
            }
        };

        Ok(Some(output))
    }

    pub(crate) fn surface_config(&self) -> &SurfaceConfiguration {
        &self.surface_config
    }

    pub(crate) fn window(&self) -> &Window {
        &self.window
    }
}
