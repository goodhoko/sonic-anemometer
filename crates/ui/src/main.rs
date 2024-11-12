use std::{
    ops::Deref,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use audio_anemometer::{
    computer::{Computer, SimpleComputer},
    simulator::Simulator,
};

use winit::{
    event::{Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
    window::Window,
};

const COMPARISON_WINDOW_WIDTH: usize = 1024;
const MAX_EXPECTED_DELAY_SAMPLES: usize = 2048;

// TODO: make these dynamically changeable by the user by winit key events
const DELAY_SAMPLES: usize = 999;
const ATTENUATION: f32 = 0.5;
const SIGNAL_TO_NOISE_RATIO: f32 = 5.0;

fn main() {
    let simulator = Arc::new(RwLock::new(Simulator::new(
        DELAY_SAMPLES,
        ATTENUATION,
        SIGNAL_TO_NOISE_RATIO,
    )));
    let computer = Arc::new(RwLock::new(SimpleComputer::new(
        MAX_EXPECTED_DELAY_SAMPLES,
        COMPARISON_WINDOW_WIDTH,
    )));

    spawn_audio_pipeline_simulation(&computer, &simulator);
    // TODO: run GUI instead
    // run_tui(computer);
    run_gui(computer);
}

/// Spawn a thread that advances the simulator and the computer.
fn spawn_audio_pipeline_simulation(
    computer: &Arc<RwLock<SimpleComputer>>,
    simulator: &Arc<RwLock<Simulator>>,
) {
    let computer = Arc::clone(computer);
    let simulator = Arc::clone(simulator);

    thread::spawn(move || {
        let mut samples = 0;
        let mut last_report = Instant::now();
        loop {
            let output_sample = computer.write().unwrap().output_sample();
            let input_sample = simulator.write().unwrap().tick(output_sample);
            computer.write().unwrap().record_sample(input_sample);

            samples += 1;

            if last_report.elapsed() > Duration::from_secs(1) {
                println!("processed {samples} samples");
                samples = 0;
                last_report = Instant::now();
            }
        }
    });
}

// TODO:
// - init winit, wgpu, etc.
// - start event loop
// - on RedrawRequested
//   - lock, clone, unlock the computer
//   - compute delay
//   - enqueue the computer's buffers, and the delay as uniforms to a render pass
//   - run the render pass and present
//   - ask for another redraw
//
// - on certain key press change the params of the simulator (delay, signal2noise ration, attenuation)
fn run_gui(computer: Arc<RwLock<SimpleComputer>>) {
    let event_loop = EventLoop::new().unwrap();
    let window = winit::window::WindowBuilder::new()
        .with_title("Audio-anemometer Visualization")
        .with_inner_size(winit::dpi::LogicalSize::new(900, 900))
        .build(&event_loop)
        .unwrap();

    pollster::block_on(run(event_loop, window));
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut size = window.inner_size();
    size.width = size.width.max(1);
    size.height = size.height.max(1);

    let instance = wgpu::Instance::default();

    let surface = instance.create_surface(&window).unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    // Load the shaders from disk
    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let swapchain_format = surface.get_capabilities(&adapter).formats[0];

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
        primitive: wgpu::PrimitiveState::default(),
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
        let _ = (&instance, &adapter, &shader, &pipeline_layout);

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
                // On macos the window needs to be redrawn manually after resizing
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let frame = surface
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
                render_pass.draw(0..3, 0..1);
                drop(render_pass);

                queue.submit(Some(encoder.finish()));
                frame.present();
            }
            WindowEvent::CloseRequested => target.exit(),
            _ => {}
        };
    });

    res.unwrap();
}

#[expect(unused)]
fn run_tui(computer: Arc<RwLock<SimpleComputer>>) {
    let mut accumulated_delay = 0;
    let mut delays = 0;
    let mut last_report = Instant::now();
    loop {
        // Computing the delay() is much more expensive than cloning the entire computer.
        // To lower lock contention, copy a snapshot of the computer to this thread
        // and immediately release the lock.
        let computer = computer.read().unwrap().deref().clone();

        if let Some(delay) = computer.delay() {
            accumulated_delay += delay;
            delays += 1;

            if last_report.elapsed() > Duration::from_secs(1) {
                let avg = accumulated_delay as f32 / delays as f32;
                println!("delay {avg} samples (averaged over {delays} computations)");
                accumulated_delay = 0;
                delays = 0;
                last_report = Instant::now();
            }
        } else {
            // The computer is not ready yet. Give it some time to accumulate more samples.
            thread::sleep(Duration::from_millis(100));
        }
    }
}
