use wgpu::{PresentMode, StorageTextureAccess, TextureFormat};

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
    pub(crate) workgroup_size: [u32; 3],
    pub(crate) kernels: Vec<KernelDescriptor>,
    pub(crate) user_bindings: Vec<UserBinding>,
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

pub enum UserResource {
    Buffer(wgpu::Buffer),
    Texture(wgpu::TextureView),
    Sampler(wgpu::Sampler),
}

pub struct UserBinding {
    pub binding: u32,
    pub resource: UserResource,
}

#[derive(Clone, Copy, Debug)]
pub enum KernelResourceType {
    StorageTexture { write: bool },
    SampledTexture,
    Sampler,
    UniformBuffer,
    StorageBuffer { read_only: bool },
}

pub struct BindingDescriptor {
    pub binding: u32,
    pub name: &'static str,
    pub resource_type: KernelResourceType,
}

pub struct KernelDescriptor {
    pub name: &'static str,
    pub entry_point: &'static str,
    pub workgroup_size: [u32; 3],
    pub bindings: &'static [BindingDescriptor],
}
