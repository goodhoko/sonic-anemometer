use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

use eyre::{Context, Ok, Result};

use crate::{computer::Computer, simulator::Simulator};
use wgpu::{
    util::DeviceExt, BindGroupDescriptor, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Device,
    Extent3d, Instance, PrimitiveTopology, ShaderStages, Texture, TextureView,
};
use winit::{
    event::{Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

pub fn run_gui(
    computer: Arc<RwLock<Computer>>,
    simulator: Option<Arc<RwLock<Simulator>>>,
) -> Result<()> {
    let event_loop = EventLoop::new().wrap_err("creating event loop<")?;
    let window = winit::window::WindowBuilder::new()
        .with_title("Audio-anemometer Visualization")
        .with_inner_size(winit::dpi::LogicalSize::new(1600, 800))
        .build(&event_loop)
        .wrap_err("creating GUI window")?;

    pollster::block_on(run(event_loop, window, computer, simulator));

    Ok(())
}

async fn run(
    event_loop: EventLoop<()>,
    window: Window,
    computer: Arc<RwLock<Computer>>,
    simulator: Option<Arc<RwLock<Simulator>>>,
) {
    let mut size = window.inner_size();
    size.width = size.width.max(1);
    size.height = size.height.max(1);

    let instance = wgpu::Instance::default();
    let (surface, adapter) = get_surface_and_adapter(instance, &window).await;

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let horizontal_size = computer.read().unwrap().output_buffer().capacity();
    let (horizontal_texture, horizontal_texture_size, horizontal_texture_view) =
        get_1d_texture_size_view(&device, horizontal_size);

    let vertical_size = computer.read().unwrap().input_buffer().capacity();
    let (vertical_texture, vertical_texture_size, vertical_texture_view) =
        get_1d_texture_size_view(&device, vertical_size);

    let sampler = device.create_sampler(&Default::default());

    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: &0.0f32.to_le_bytes(),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D1,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D1,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(4),
                },
                count: None,
            },
        ],
    });
    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&horizontal_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&vertical_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let swapchain_format = surface.get_capabilities(&adapter).formats[0];

    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(swapchain_format.into())],
        }),
        primitive: wgpu::PrimitiveState {
            topology: PrimitiveTopology::TriangleStrip,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    let mut config = surface
        .get_default_config(&adapter, size.width, size.height)
        .unwrap();
    surface.configure(&device, &config);

    if simulator.is_some() {
        println!("Use keys to tweak simulator params:");
        println!("A/S to increase/decrease gain");
        println!("D/F to increase/decrease delay");
        println!("N/M to decrease/increase signal to noise ratio");
    }

    let window = &window;
    let res = event_loop.run(move |event, target| {
        // Have the closure take ownership of the resources.
        // `event_loop.run` never returns, therefore we must do this to ensure
        // the resources are properly cleaned up.
        let _ = (&adapter, &shader, &pipeline_layout);

        let Event::WindowEvent {
            window_id: _,
            event,
        } = event
        else {
            return;
        };

        match event {
            WindowEvent::Resized(new_size) => {
                // Reconfigure the surface with the new size
                config.width = new_size.width.max(1);
                config.height = new_size.height.max(1);
                surface.configure(&device, &config);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        text: Some(pressed_str),
                        ..
                    },
                ..
            } => {
                if let Some(simulator) = simulator.as_ref() {
                    if pressed_str == "a" {
                        let mut simulator = simulator.write().unwrap();
                        simulator.gain *= 1.1;
                        println!("gain: {}", simulator.gain);
                    } else if pressed_str == "s" {
                        let mut simulator = simulator.write().unwrap();
                        simulator.gain *= 0.9;
                        println!("gain: {}", simulator.gain);
                    } else if pressed_str == "n" {
                        let mut simulator = simulator.write().unwrap();
                        simulator.signal_to_noise_ratio *= 0.9;
                        println!("signal to noise ratio: {}", simulator.signal_to_noise_ratio);
                    } else if pressed_str == "m" {
                        let mut simulator = simulator.write().unwrap();
                        simulator.signal_to_noise_ratio *= 1.1;
                        println!("signal to noise ratio: {}", simulator.signal_to_noise_ratio);
                    } else if pressed_str == "d" {
                        let mut simulator = simulator.write().unwrap();
                        let delay = simulator.delay_samples().saturating_add(5);
                        simulator.set_delay(delay);
                        println!("delay: {}", delay);
                    } else if pressed_str == "f" {
                        let mut simulator = simulator.write().unwrap();
                        let delay = simulator.delay_samples().saturating_sub(5);
                        simulator.set_delay(delay);
                        println!("delay: {}", delay);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let computer = computer.read().unwrap().deref().clone();
                let hor_samples = computer
                    .output_buffer()
                    .iter()
                    .flat_map(|item| item.to_le_bytes())
                    .collect::<Vec<_>>();
                let ver_samples = computer
                    .input_buffer()
                    .iter()
                    .flat_map(|item| item.to_le_bytes())
                    .collect::<Vec<_>>();

                queue.write_texture(
                    // Tells wgpu where to copy the data
                    wgpu::ImageCopyTexture {
                        texture: &horizontal_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    // The actual data
                    &hor_samples,
                    // The layout of the texture
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(hor_samples.len() as u32),
                        rows_per_image: Some(1),
                    },
                    horizontal_texture_size,
                );

                queue.write_texture(
                    // Tells wgpu where to copy the data
                    wgpu::ImageCopyTexture {
                        texture: &vertical_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    // The actual data
                    &ver_samples,
                    // The layout of the texture
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(ver_samples.len() as u32),
                        rows_per_image: Some(1),
                    },
                    vertical_texture_size,
                );

                let delay_samples = computer.delay().unwrap_or(0) as f32;
                let delay_relative = 1.0 - delay_samples / horizontal_size as f32;
                queue.write_buffer(&uniform_buffer, 0, &delay_relative.to_le_bytes());

                let frame: wgpu::SurfaceTexture = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                render_pass.set_pipeline(&render_pipeline);
                render_pass.set_bind_group(0, &bind_group, &[]);
                render_pass.draw(0..4, 0..1);
                drop(render_pass);

                queue.submit(Some(encoder.finish()));
                frame.present();

                window.request_redraw();
            }
            WindowEvent::CloseRequested => target.exit(),
            _ => {}
        };
    });

    res.unwrap();
}

async fn get_surface_and_adapter(
    instance: Instance,
    window: &Window,
) -> (wgpu::Surface, wgpu::Adapter) {
    let surface = instance.create_surface(window).unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    (surface, adapter)
}

fn get_1d_texture_size_view(device: &Device, size: usize) -> (Texture, Extent3d, TextureView) {
    let texture_size = wgpu::Extent3d {
        width: size as u32,
        height: 1,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D1,
        format: wgpu::TextureFormat::R32Float,
        // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
        // COPY_DST means that we want to copy data to this texture
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("horizontal texture"),
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    (texture, texture_size, view)
}
