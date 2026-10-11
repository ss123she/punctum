use wgpu::PresentMode;
use winit::event_loop::EventLoop;

use crate::runner::Runner;

const DEFAULT_SHADER: &'static str = r#"
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

// VS
@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

// FS
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}
"#;

#[derive(Debug, Clone)]
pub(crate) struct WindowConfig {
    pub(crate) title: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderConfig {
    pub(crate) present_mode: PresentMode,
    pub(crate) shader_text: String,
}

pub(crate) struct AppConfig {
    pub(crate) window: WindowConfig,
    pub(crate) render: RenderConfig,
}

pub struct Builder {
    window: WindowConfig,
    render: RenderConfig,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            window: WindowConfig {
                title: "Punctum".to_string(),
                width: 1366,
                height: 768,
            },
            render: RenderConfig {
                present_mode: PresentMode::Fifo,
                shader_text: DEFAULT_SHADER.to_string(),
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

    pub fn with_present_mode(mut self, present_mode: PresentMode) -> Self {
        self.render.present_mode = present_mode;
        self
    }

    fn build(self) -> AppConfig {
        AppConfig {
            window: self.window,
            render: self.render,
        }
    }

    pub fn run(self) -> anyhow::Result<()> {
        let config = self.build();

        #[cfg(not(target_arch = "wasm32"))]
        env_logger::init();

        #[cfg(target_arch = "wasm32")]
        console_log::init_with_level(log::Level::Info).unwrap_throw();

        let event_loop = EventLoop::with_user_event().build()?;
        let mut runner = Runner::new(config, &event_loop);

        #[cfg(not(target_arch = "wasm32"))]
        event_loop.run_app(&mut runner)?;

        #[cfg(target_arch = "wasm32")]
        event_loop.spawn_app(runner);

        Ok(())
    }
}
