use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

use eyre::{Context, Ok, Result};
use self_similarity_matrix::SelfSimilarityMatrix;

use crate::{computer::Computer, simulator::Simulator};
use wgpu::Instance;
use winit::{
    event::{Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

mod self_similarity_matrix;

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

    let swapchain_format = surface.get_capabilities(&adapter).formats[0];
    let visualization = SelfSimilarityMatrix::new(
        computer.read().unwrap().output_buffer().capacity(),
        computer.read().unwrap().input_buffer().capacity(),
        &device,
        swapchain_format,
    );

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
        let _ = (&adapter, &visualization);

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
                let delay_samples = computer.delay().map(|res| res.delay_samples).unwrap_or(0);

                let frame: wgpu::SurfaceTexture = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let commands = visualization.render(
                    computer.output_buffer().iter(),
                    computer.input_buffer().iter(),
                    delay_samples,
                    &queue,
                    view,
                    &device,
                );

                queue.submit(Some(commands));
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
