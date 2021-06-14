use std::borrow::Cow;
use std::sync::Arc;

use wgpu::CommandBuffer;
use wgpu::ShaderModule;

pub struct ScreenTargets {
    pub extent: wgpu::Extent3d,
    pub color: Arc<wgpu::SwapChainFrame>,
    pub depth: Arc<wgpu::TextureView>,
}

// let render_command_buffer = app.draw(&device, targets, &spawner);
pub async fn render_stuff(
    device: &wgpu::Device,
    targets: Arc<ScreenTargets>,
    render_pipeline: &wgpu::RenderPipeline,
) -> wgpu::CommandBuffer {
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &targets.color.output.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
        rpass.set_pipeline(&render_pipeline);
        rpass.draw(0..3, 0..1);
    }
    encoder.finish()
}

pub struct Autonomy {
    shader: ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    render_pipeline: wgpu::RenderPipeline,
}

impl Autonomy {
    pub fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../res/shader/main.wgsl"))),
            flags: wgpu::ShaderFlags::all(),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[color_format.into()],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        });

        Autonomy {
            shader,
            pipeline_layout,
            render_pipeline,
        }
    }

    pub async fn draw(&self, device: &wgpu::Device, targets: Arc<ScreenTargets>) -> Vec<CommandBuffer> {
        vec![render_stuff(device, targets, &self.render_pipeline).await]
    }
}
