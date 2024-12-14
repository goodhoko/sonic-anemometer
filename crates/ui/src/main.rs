use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

use audio_anemometer::{
    computer::Computer,
    simulator::{simulate_audio_pipeline, Simulator},
};

use wgpu::{
    util::DeviceExt, BindGroupDescriptor, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Device,
    Extent3d, Instance, PrimitiveTopology, ShaderStages, Texture, TextureView,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

const COMPARISON_WINDOW_WIDTH: usize = 1024;
const MAX_EXPECTED_DELAY_SAMPLES: usize = 2048;

// TODO: make these dynamically changeable by winit key events.
const DELAY_SAMPLES: usize = 333;
const ATTENUATION: f32 = 1.0;
const SIGNAL_TO_NOISE_RATIO: f32 = 10.0;

fn main() {
    let simulator = Arc::new(RwLock::new(Simulator::new(
        DELAY_SAMPLES,
        ATTENUATION,
        SIGNAL_TO_NOISE_RATIO,
    )));
    let computer = Arc::new(RwLock::new(Computer::new(
        MAX_EXPECTED_DELAY_SAMPLES,
        COMPARISON_WINDOW_WIDTH,
    )));

    simulate_audio_pipeline(&computer, &simulator);

    let event_loop = EventLoop::new().unwrap();
    let window = winit::window::WindowBuilder::new()
        .with_title("Audio-anemometer Visualization")
        .with_inner_size(winit::dpi::LogicalSize::new(1600, 800))
        .build(&event_loop)
        .unwrap();

    pollster::block_on(run(event_loop, window, computer));
}

async fn run(event_loop: EventLoop<()>, window: Window, computer: Arc<RwLock<Computer>>) {
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

    let horizontal_size = MAX_EXPECTED_DELAY_SAMPLES + COMPARISON_WINDOW_WIDTH;
    let (horizontal_texture, horizontal_texture_size, horizontal_texture_view) =
        get_1d_texture_size_view(&device, horizontal_size);

    let vertical_size = COMPARISON_WINDOW_WIDTH;
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
