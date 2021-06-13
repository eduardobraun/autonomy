use std::sync::Arc;

use wgpu::CommandBuffer;

pub struct ScreenTargets {
    pub extent: wgpu::Extent3d,
    pub color: Arc<wgpu::SwapChainFrame>,
    pub depth: Arc<wgpu::TextureView>,
}

// let render_command_buffer = app.draw(&device, targets, &spawner);
pub async fn render_stuff(
    device: &wgpu::Device,
    targets: Arc<ScreenTargets>,
) -> wgpu::CommandBuffer {
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        // rpass.set_pipeline(&render_pipeline);
        // rpass.draw(0..3, 0..1);
    }
    encoder.finish()
}

pub struct Autonomy {}

impl Autonomy {
    pub fn new() -> Self {
        Autonomy {}
    }

    pub async fn draw(&self, device: &wgpu::Device, targets: Arc<ScreenTargets>) -> CommandBuffer {
        render_stuff(device, targets).await
    }
}
