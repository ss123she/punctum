use wgpu::PresentMode;
use winit::event_loop::EventLoop;

use crate::{
    AppConfig, ComputeConfig, KernelDescriptor, RenderConfig, UpdateContext, UserBinding,
    UserResource, WindowConfig, app::runner::Runner, configs::UpdateFn,
};

const DEFAULT_RENDER_SHADER: &'_ str = include_str!("pipeline/default_render.wgsl");
const DEFAULT_COMPUTE_SPV: &'_ [u8] = include_bytes!(env!("COMPUTE_SPV"));

/// Builder for configuring and launching a Punctum app.
pub struct Builder {
    window: WindowConfig,
    render: RenderConfig,
    compute: ComputeConfig,
    on_update: Option<UpdateFn>,
}

impl Default for Builder {
    /// Creates a builder with a 1366x768 window, Fifo present mode, and the built-in shaders.
    fn default() -> Self {
        Self {
            window: WindowConfig {
                title: "Punctum".to_string(),
                width: 1366,
                height: 768,
                canvas_id: None,
            },
            render: RenderConfig {
                present_mode: PresentMode::Fifo,
                shader_text: DEFAULT_RENDER_SHADER.to_string(),
            },
            compute: ComputeConfig {
                shader_bytes: DEFAULT_COMPUTE_SPV.to_vec(),
                entry_point: "main".to_string(),
                workgroup_size: [16, 16, 1],
                kernels: Vec::new(),
                user_bindings: Vec::new(),
                pending_uniforms: Vec::new(),
            },
            on_update: None,
        }
    }
}

/// Creates a new [`Builder`] with default settings.
pub fn builder() -> Builder {
    Builder::default()
}

impl Builder {
    /// Sets the window title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.window.title = title.into();
        self
    }

    /// Sets the initial window size in pixels.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.window.width = width;
        self.window.height = height;
        self
    }

    /// Sets the HTML canvas element id to render into (web targets).
    pub fn with_canvas_id(mut self, id: impl Into<String>) -> Self {
        self.window.canvas_id = Some(id.into());
        self
    }

    /// Enables or disables vsync using the automatic present modes.
    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.render.present_mode = if vsync {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        };
        self
    }

    /// Sets an explicit surface present mode.
    pub fn with_present_mode(mut self, present_mode: PresentMode) -> Self {
        self.render.present_mode = present_mode;
        self
    }

    /// Replaces the compute shader with the given SPIR-V bytes.
    pub fn with_spv(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.compute.shader_bytes = bytes.into();
        self
    }

    /// Registers a kernel and makes it the active entry point and workgroup size.
    pub fn with_kernel(mut self, kernel: KernelDescriptor) -> Self {
        self.compute.entry_point = kernel.entry_point.to_string();
        self.compute.workgroup_size = kernel.workgroup_size;
        self.compute.kernels.push(kernel);
        self
    }

    /// Uses a custom compute shader with the given entry point and workgroup size.
    pub fn with_custom_compute(
        mut self,
        bytes: Vec<u8>,
        entry_point: impl Into<String>,
        workgroup_size: [u32; 3],
    ) -> Self {
        self.compute.shader_bytes = bytes;
        self.compute.entry_point = entry_point.into();
        self.compute.workgroup_size = workgroup_size;
        self
    }

    /// Binds a user buffer at the given binding, replacing any existing one.
    pub fn with_buffer(mut self, binding: u32, buffer: wgpu::Buffer) -> Self {
        self.compute.user_bindings.retain(|b| b.binding != binding);
        self.compute.user_bindings.push(UserBinding {
            binding,
            resource: UserResource::Buffer(buffer),
        });
        self
    }

    /// Binds a user texture view at the given binding, replacing any existing one.
    pub fn with_texture(mut self, binding: u32, view: wgpu::TextureView) -> Self {
        self.compute.user_bindings.retain(|b| b.binding != binding);
        self.compute.user_bindings.push(UserBinding {
            binding,
            resource: UserResource::Texture(view),
        });
        self
    }

    /// Binds a user sampler at the given binding, replacing any existing one.
    pub fn with_sampler(mut self, binding: u32, sampler: wgpu::Sampler) -> Self {
        self.compute.user_bindings.retain(|b| b.binding != binding);
        self.compute.user_bindings.push(UserBinding {
            binding,
            resource: UserResource::Sampler(sampler),
        });
        self
    }

    /// Adds a uniform at the given binding, initialized from the raw bytes of `initial_value`.
    pub fn with_uniform<T: Copy + 'static>(mut self, binding: u32, initial_value: &T) -> Self {
        let bytes = unsafe {
            std::slice::from_raw_parts(
                initial_value as *const T as *const u8,
                std::mem::size_of::<T>(),
            )
        };
        self.compute
            .pending_uniforms
            .push(crate::configs::PendingUniform {
                binding,
                data: bytes.to_vec(),
            });
        self
    }

    /// Sets a callback invoked on every update tick.
    pub fn on_update<F>(mut self, f: F) -> Self
    where
        F: FnMut(&UpdateContext) + 'static,
    {
        self.on_update = Some(Box::new(f));
        self
    }

    /// Consumes the builder and produces the final [`AppConfig`].
    fn build(self) -> AppConfig {
        AppConfig {
            window: self.window,
            render: self.render,
            compute: self.compute,
            on_update: self.on_update,
        }
    }

    /// Builds the config, creates the event loop, and runs the app.
    pub fn run(self) -> anyhow::Result<()> {
        let config = self.build();

        #[cfg(not(target_arch = "wasm32"))]
        env_logger::init();

        let event_loop = EventLoop::with_user_event().build()?;

        #[allow(unused_mut)]
        let mut runner = Runner::new(config, &event_loop);

        #[cfg(not(target_arch = "wasm32"))]
        event_loop.run_app(&mut runner)?;

        Ok(())
    }
}
