use crate::core::GpuContext;

pub struct ComputePipeline {
    pub(crate) compute_pipeline: wgpu::ComputePipeline,
}

pub struct ComputeConfig {
    pub shader_text: String,
    pub entry_point: String,
}

impl ComputePipeline {
    pub fn new(gpu: &GpuContext, config: &ComputeConfig) -> anyhow::Result<Self> {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(config.shader_text.as_str().into()),
            });

        let layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        let compute_pipeline =
            gpu.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute Pipeline"),
                    layout: Some(&layout),
                    module: &shader,
                    entry_point: Some(&config.entry_point),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                });

        Ok(Self { compute_pipeline })
    }

    pub fn dispatch(&self, cpass: &mut wgpu::ComputePass, x: u32, y: u32, z: u32) {
        cpass.set_pipeline(&self.compute_pipeline);
        cpass.dispatch_workgroups(x, y, z);
    }
}
