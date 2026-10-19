use wgpu::PresentMode;

#[derive(Debug, Clone)]
pub(crate) struct WindowConfig {
    pub(crate) title: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) canvas_id: Option<String>,
}

pub(crate) struct ComputeConfig {
    pub(crate) shader_bytes: Vec<u8>,
    pub(crate) entry_point: String,
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

pub struct KernelDescriptor {
    pub name: &'static str,
    pub entry_point: &'static str,
    pub workgroup_size: [u32; 3],
}
