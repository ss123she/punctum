use wgpu::{
    BindGroupLayoutEntry, BindingType, ShaderStages, StorageTextureAccess, TextureFormat,
    TextureViewDimension,
};

use crate::{ComputeConfig, core::GpuContext};

pub struct ComputePipeline {
    pub(crate) compute_pipeline: wgpu::ComputePipeline,
}

impl ComputePipeline {
    pub(crate) fn new(gpu: &GpuContext, config: &ComputeConfig) -> anyhow::Result<Self> {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(config.shader_text.as_str().into()),
            });

        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Compute Bind Group Layout"),
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba8Unorm,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    }],
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

    pub fn create_bind_group(&self, gpu: &GpuContext, view: &wgpu::TextureView) -> wgpu::BindGroup {
        let layout = self.compute_pipeline.get_bind_group_layout(0);

        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            }],
        })
    }
}
