use wgpu::PresentMode;

#[derive(Debug, Clone)]
pub(crate) struct WindowConfig {
    pub(crate) title: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) canvas_id: Option<String>,
}

pub struct ComputeConfig {
    pub shader_text: String,
    pub entry_point: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderConfig {
    pub(crate) present_mode: PresentMode,
    pub(crate) shader_text: String,
}

pub(crate) struct AppConfig {
    pub(crate) window: WindowConfig,
    pub(crate) render: RenderConfig,
    pub(crate) compute: ComputeConfig,
}
