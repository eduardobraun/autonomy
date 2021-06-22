use std::borrow::Cow;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroup, BindGroupLayout, CommandBuffer, util::DeviceExt};

pub mod camera;
pub(crate) mod helpers;
pub mod terrain;
use self::terrain::Terrain;
use self::camera::Camera;

pub struct ScreenTargets {
    pub extent: wgpu::Extent3d,
    pub color: Arc<wgpu::SwapChainFrame>,
    pub depth: Arc<wgpu::TextureView>,
}

pub async fn clear_screen(
    device: &wgpu::Device,
    targets: Arc<ScreenTargets>,
    color: wgpu::Color,
) -> wgpu::CommandBuffer {
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &targets.color.output.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
    }
    encoder.finish()
}

pub struct Triangle {
    render_pipeline: wgpu::RenderPipeline,
}

impl Triangle {
    pub fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat, uniforms_bgl: &BindGroupLayout) -> Self {
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../res/shader/main.wgsl"))),
            flags: wgpu::ShaderFlags::all(),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[uniforms_bgl],
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

        Self { render_pipeline }
    }

    pub async fn draw(&self, device: &wgpu::Device, targets: Arc<ScreenTargets>, uniforms_bg: &BindGroup) -> CommandBuffer {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &targets.color.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, uniforms_bg, &[]);
            rpass.draw(0..3, 0..1);
        }
        encoder.finish()
    }
}

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone)]
struct Uniforms {
    // We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

unsafe impl Zeroable for Uniforms{}
unsafe impl Pod for Uniforms{}

impl Uniforms {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}


pub struct Autonomy {
    camera: Camera,
    triangle: Triangle,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: BindGroup,
    terrain: Terrain,
}

impl Autonomy {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, color_format: wgpu::TextureFormat) -> Self {
        let camera = Camera{
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: 1.0,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);

        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            }
        );

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("uniform_bind_group_layout"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("uniform_bind_group"),
        });

        let triangle = Triangle::new(device, color_format, &uniform_bind_group_layout);
        let terrain = Terrain::new(device, queue, color_format, &uniform_bind_group_layout);
        Autonomy {
            camera,
            triangle,
            terrain,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    pub async fn draw(&self, device: &wgpu::Device, targets: Arc<ScreenTargets>) -> Vec<CommandBuffer> {
        // TODO: should use spawn
        let f1 = clear_screen(device, targets.clone(), wgpu::Color{ r: 0.2, g: 0.2, b: 0.2, a: 1.0 });
        let f2 = self.triangle.draw(device, targets.clone(), &self.uniform_bind_group);
        let f3 = self.terrain.draw(device, targets.clone(), &self.uniform_bind_group);
        let (b1, b2, b3) = futures::join!(f1,f2, f3);
        vec![b1, b2, b3]
    }
}
