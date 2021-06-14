use autonomy::{Autonomy, ScreenTargets};
use futures::executor::LocalPool;
use winit::{
    event,
    event_loop::{ControlFlow, EventLoop},
};
use env_logger;
use log::info;

use std::sync::Arc;

fn main() {
    env_logger::init();
    main_loop();
}

pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
use profiling;

#[profiling::function]
pub fn main_loop() {
    use std::time;
    let event_loop = EventLoop::new();
    let window = Arc::new(winit::window::Window::new(&event_loop).unwrap());
    let mut task_pool = LocalPool::new();

    let size = window.inner_size();
    let mut extent = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
    };
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&*window) };

    let adapter = task_pool
            .run_until(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            }))
            .expect("Unable to initialize GPU via the selected backend.");

        let limits = wgpu::Limits::default();
        let (device, queue) = {
            let (d, q) = task_pool
            .run_until(adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits,
                },
                None,
            ))
            .unwrap();
            (Arc::new(d), Arc::new(q))
        };

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: COLOR_FORMAT,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);
        let mut depth_target = Arc::new(device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth"),
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            })
            .create_view(&wgpu::TextureViewDescriptor::default()));


        let mut last_time = time::Instant::now();
        let mut needs_reload = false;

        let app = Autonomy::new(&device, COLOR_FORMAT);

        event_loop.run(move |event, _, control_flow| {
            let _ = window;
            *control_flow = ControlFlow::Poll;
            task_pool.run_until_stalled();

            match event {
                event::Event::WindowEvent {
                    event: event::WindowEvent::Resized(size),
                    ..
                } => {
                    info!("Resizing to {:?}", size);
                    extent = wgpu::Extent3d {
                        width: size.width,
                        height: size.height,
                        depth_or_array_layers: 1,
                    };
                    let sc_desc = wgpu::SwapChainDescriptor {
                        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
                        format: COLOR_FORMAT,
                        width: size.width,
                        height: size.height,
                        present_mode: wgpu::PresentMode::Mailbox,
                    };
                    swap_chain = device.create_swap_chain(&surface, &sc_desc);
                    depth_target = Arc::new(device
                        .create_texture(&wgpu::TextureDescriptor {
                            label: Some("Depth"),
                            size: extent,
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: DEPTH_FORMAT,
                            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
                        })
                        .create_view(&wgpu::TextureViewDescriptor::default()));
                    // app.resize(&device, extent);
                }
                event::Event::WindowEvent { event, .. } => match event {
                    event::WindowEvent::Focused(false) => {
                        needs_reload = true;
                    }
                    event::WindowEvent::Focused(true) if needs_reload => {
                        info!("Reloading shaders");
                        // app.reload(&device);
                        needs_reload = false;
                    }
                    event::WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    event::WindowEvent::KeyboardInput { input, .. } => {
                        // if !app.on_key(input) {
                        //     *control_flow = ControlFlow::Exit;
                        // }
                    }
                    // event::WindowEvent::MouseWheel { delta, .. } => app.on_mouse_wheel(delta),
                    event::WindowEvent::CursorMoved { position, .. } => {
                        // app.on_cursor_move(position.into())
                    }
                    event::WindowEvent::MouseInput { state, button, .. } => {
                        // app.on_mouse_button(state, button)
                    }
                    _ => {}
                },
                event::Event::MainEventsCleared => {
                    let _spawner = task_pool.spawner();
                    let duration = time::Instant::now() - last_time;
                    last_time += duration;
                    let _delta = duration.as_secs() as f32 + duration.subsec_nanos() as f32 * 1.0e-9;

                    // let update_command_buffers = app.update(&device, delta, &spawner);
                    // if !update_command_buffers.is_empty() {
                    //     queue.submit(update_command_buffers);
                    // }

                    match swap_chain.get_current_frame() {
                        Ok(frame) => {
                            let frame = Arc::new(frame);
                            let targets = Arc::new(ScreenTargets {
                                extent,
                                color: frame.clone(),
                                depth: depth_target.clone(),
                            });
                            let render_command_buffer = task_pool.run_until(app.draw(&device, targets));
                            queue.submit(render_command_buffer);
                        }
                        Err(_) => {}
                    };

                    profiling::finish_frame!();
                }
                _ => (),
            }
        });
    }
