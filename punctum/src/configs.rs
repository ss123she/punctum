use wgpu::{PresentMode, StorageTextureAccess, TextureFormat};

#[derive(Debug, Clone)]
pub(crate) struct WindowConfig {
    pub(crate) title: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) canvas_id: Option<String>,
}

#[derive(Clone)]
pub struct PendingUniform {
    pub binding: u32,
    pub data: Vec<u8>,
}

pub(crate) struct ComputeConfig {
    pub(crate) shader_bytes: Vec<u8>,
    pub(crate) entry_point: String,
    pub(crate) workgroup_size: [u32; 3],
    pub(crate) kernels: Vec<KernelDescriptor>,
    pub(crate) user_bindings: Vec<UserBinding>,
    pub(crate) pending_uniforms: Vec<PendingUniform>,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderConfig {
    pub(crate) present_mode: PresentMode,
    pub(crate) shader_text: String,
}

pub struct UpdateContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub delta_time: f32,
    pub total_time: f32,
    pub(crate) user_bindings: &'a [UserBinding],
    pub mouse: [f32; 2],
    pub mouse_down: bool,
}

impl<'a> UpdateContext<'a> {
    pub fn update_buffer<T: Copy>(&self, binding: u32, value: &T) {
        let bytes = unsafe {
            std::slice::from_raw_parts(value as *const T as *const u8, std::mem::size_of::<T>())
        };
        for ub in self.user_bindings {
            if ub.binding == binding {
                if let UserResource::Buffer(buf) = &ub.resource {
                    self.queue.write_buffer(buf, 0, bytes);
                }
                return;
            }
        }
    }
}

pub type UpdateFn = Box<dyn FnMut(&UpdateContext) + 'static>;

pub(crate) struct AppConfig {
    pub(crate) window: WindowConfig,
    pub(crate) render: RenderConfig,
    pub(crate) compute: ComputeConfig,
    pub(crate) on_update: Option<UpdateFn>,
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
