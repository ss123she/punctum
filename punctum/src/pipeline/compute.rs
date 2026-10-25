use std::borrow::Cow;

use wgpu::{
    BindGroupEntry, BindGroupLayoutEntry, BindingType, PassthroughShaderEntryPoint, ShaderStages,
    StorageTextureAccess, TextureFormat, TextureViewDimension,
};

use crate::{
    ComputeConfig, KernelResourceType,
    configs::{UserBinding, UserResource},
    core::GpuContext,
};

pub struct ComputePipeline {
    pub(crate) compute_pipeline: wgpu::ComputePipeline,
}

impl ComputePipeline {
    pub(crate) fn new(gpu: &GpuContext, config: &ComputeConfig) -> anyhow::Result<Self> {
        let shader = unsafe {
            gpu.device
                .create_shader_module_passthrough(wgpu::ShaderModuleDescriptorPassthrough {
                    label: Some("rust-gpu Compute Shader"),
                    spirv: Some(wgpu::util::make_spirv_raw(&config.shader_bytes)),

                    entry_points: Cow::Borrowed(&[PassthroughShaderEntryPoint {
                        name: Cow::Borrowed(&config.entry_point),
                        workgroup_size: (
                            config.workgroup_size[0],
                            config.workgroup_size[1],
                            config.workgroup_size[2],
                        ),
                    }]),

                    ..Default::default()
                })
        };

        let active_kernel = config
            .kernels
            .iter()
            .find(|k| k.entry_point == config.entry_point);

        let entries: Vec<BindGroupLayoutEntry> = match active_kernel {
            Some(kernel) => kernel
                .bindings
                .iter()
                .map(|b| BindGroupLayoutEntry {
                    binding: b.binding,
                    visibility: ShaderStages::COMPUTE,
                    ty: match b.resource_type {
                        KernelResourceType::StorageTexture { write } => {
                            BindingType::StorageTexture {
                                access: if write {
                                    StorageTextureAccess::WriteOnly
                                } else {
                                    StorageTextureAccess::ReadOnly
                                },
                                format: TextureFormat::Rgba8Unorm,
                                view_dimension: TextureViewDimension::D2,
                            }
                        }
                        KernelResourceType::SampledTexture => BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        KernelResourceType::Sampler => {
                            BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
                        }
                        KernelResourceType::UniformBuffer => BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        KernelResourceType::StorageBuffer { read_only } => BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                    },
                    count: None,
                })
                .collect(),
            None => {
                vec![BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rgba8Unorm,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                }]
            }
        };

        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Compute Bind Group Layout"),
                    entries: &entries,
                });

        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });

        let compute_pipeline =
            gpu.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some(&config.entry_point),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                });

        Ok(Self { compute_pipeline })
    }

    pub(crate) fn dispatch(
        &self,
        cpass: &mut wgpu::ComputePass,
        bind_group: &wgpu::BindGroup,
        x: u32,
        y: u32,
        z: u32,
    ) {
        cpass.set_pipeline(&self.compute_pipeline);
        cpass.set_bind_group(0, bind_group, &[]);
        cpass.dispatch_workgroups(x, y, z);
    }

    pub fn create_bind_group(
        &self,
        gpu: &GpuContext,
        view: &wgpu::TextureView,
        user_bindings: &[UserBinding],
    ) -> wgpu::BindGroup {
        let layout = self.compute_pipeline.get_bind_group_layout(0);

        let mut entries = vec![BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(view),
        }];

        for ub in user_bindings {
            let resource = match &ub.resource {
                UserResource::Buffer(buf) => buf.as_entire_binding(),
                UserResource::Texture(view) => wgpu::BindingResource::TextureView(view),
                UserResource::Sampler(sampler) => wgpu::BindingResource::Sampler(sampler),
            };
            entries.push(BindGroupEntry {
                binding: ub.binding,
                resource,
            })
        }

        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &layout,
            entries: &entries,
        })
    }
}
