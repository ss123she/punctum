use wgpu::PresentMode;
use winit::event_loop::EventLoop;

use crate::{
    AppConfig, ComputeConfig, KernelDescriptor, RenderConfig, WindowConfig, app::runner::Runner,
};

// #[cfg(target_arch = "wasm32")]
// use wasm_bindgen::prelude::*;

const DEFAULT_RENDER_SHADER: &'_ str = include_str!("pipeline\\default_render.wgsl");
const DEFAULT_COMPUTE_SPV: &'_ [u8] = include_bytes!(env!("COMPUTE_SPV"));

pub struct Builder {
    window: WindowConfig,
    render: RenderConfig,
    compute: ComputeConfig,
}

impl Default for Builder {
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
            },
        }
    }
}

pub fn builder() -> Builder {
    Builder::default()
}

impl Builder {
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.window.title = title.into();
        self
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.window.width = width;
        self.window.height = height;
        self
    }

    pub fn with_canvas_id(mut self, id: impl Into<String>) -> Self {
        self.window.canvas_id = Some(id.into());
        self
    }

    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.render.present_mode = if vsync {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        };
        self
    }

    pub fn with_present_mode(mut self, present_mode: PresentMode) -> Self {
        self.render.present_mode = present_mode;
        self
    }

    pub fn with_kernel(mut self, kernel: KernelDescriptor) -> Self {
        self.compute.entry_point = kernel.entry_point.to_string();
        self.compute.workgroup_size = kernel.workgroup_size;
        self.compute.kernels.push(kernel);
        self
    }

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

    fn build(self) -> AppConfig {
        AppConfig {
            window: self.window,
            render: self.render,
            compute: self.compute,
        }
    }

    pub fn run(self) -> anyhow::Result<()> {
        let config = self.build();

        #[cfg(not(target_arch = "wasm32"))]
        env_logger::init();

        // #[cfg(target_arch = "wasm32")]
        // {
        //     use wasm_bindgen::UnwrapThrowExt;
        //     use winit::platform::web::EventLoopExtWebSys;
        //     console_error_panic_hook::set_once();
        //     let _ = console_log::init_with_level(log::Level::Info);
        // }

        let event_loop = EventLoop::with_user_event().build()?;

        #[allow(unused_mut)]
        let mut runner = Runner::new(config, &event_loop);

        #[cfg(not(target_arch = "wasm32"))]
        event_loop.run_app(&mut runner)?;

        // #[cfg(target_arch = "wasm32")]
        // {
        //     use winit::platform::web::EventLoopExtWebSys;
        //     event_loop.spawn_app(runner);
        // }

        Ok(())
    }
}

// #[cfg(target_arch = "wasm32")]
// #[wasm_bindgen]
// pub fn start(canvas_id: Option<String>) -> Result<(), wasm_bindgen::JsValue> {
//     use wasm_bindgen::UnwrapThrowExt;
//
//     let mut b = builder();
//     if let Some(id) = canvas_id {
//         b = b.with_canvas_id(id);
//     }
//     b.run().unwrap_throw();
//
//     Ok(())
// }
