use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use color_eyre::eyre::Result;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Host, SampleFormat, Stream,
};
use eyre::{eyre, Context};

use crate::computer::Computer;

pub fn run_real_world_audio(computer: Arc<RwLock<Computer>>) -> Result<(Stream, Stream)> {
    let host = cpal::default_host();

    println!("output devices:");
    host.output_devices()?.for_each(|device| {
        println!("{}", device.name().as_deref().unwrap_or("no name"));
    });

    println!("input devices:");
    host.input_devices()?.for_each(|device| {
        println!("{}", device.name().as_deref().unwrap_or("no name"));
    });

    //TODO: support selecting other devices (eg. external speakers/mic)
    let output_device = host
        .default_output_device()
        .ok_or(eyre!("getting default output device"))?;

    let input_device = host
        .default_input_device()
        .ok_or(eyre!("getting default input device"))?;

    println!(
        "choosing {} 🔊 -> 🎤 {}",
        output_device.name().as_deref().unwrap_or("no name"),
        input_device.name().as_deref().unwrap_or("no name"),
    );

    println!("supported output configs:");
    output_device
        .supported_output_configs()?
        .for_each(|config| {
            println!("{:?}", config);
        });

    println!("supported input configs:");
    input_device.supported_input_configs()?.for_each(|config| {
        println!("{:?}", config);
    });

    let input_config = input_device.default_input_config()?;
    let output_config = output_device.default_output_config()?;

    dbg!(&input_config);
    dbg!(&output_config);

    assert_eq!(input_config.sample_rate(), output_config.sample_rate());
    assert_eq!(input_config.channels(), 1);
    assert_eq!(input_config.sample_format(), SampleFormat::F32);
    assert_eq!(output_config.sample_format(), SampleFormat::F32);

    let computer_for_output = Arc::clone(&computer);
    let output_channels = output_config.channels() as usize;
    let output_stream = output_device.build_output_stream(
        &output_config.into(),
        move |output: &mut [f32], _info| {
            let mut computer = computer_for_output.write().unwrap();

            assert_eq!(output.len() % output_channels, 0);
            output
                .chunks_exact_mut(output_channels)
                .for_each(|channels| {
                    let sample = computer.output_sample();
                    channels.iter_mut().for_each(|channel| {
                        *channel = sample;
                    });
                });
        },
        |err| eprintln!("Error playing audio: {:?}", err),
        Some(Duration::from_millis(20)),
    )?;

    let computer_for_input = Arc::clone(&computer);
    let input_stream = input_device.build_input_stream(
        &input_config.into(),
        move |data: &[f32], _info| {
            // TODO: use info timestamps for more accurate delay measurement.

            let mut computer = computer_for_input.write().unwrap();
            // Copy data to shared buffer for processing
            for &sample in data.iter() {
                computer.record_sample(sample * 100.0);
            }
        },
        |err| eprintln!("Error capturing audio: {:?}", err),
        Some(Duration::from_millis(20)),
    )?;

    output_stream.play()?;
    input_stream.play()?;

    Ok((output_stream, input_stream))
}

#[allow(unused)]
fn get_input_device_or_default(host: &Host, device_name: Option<&str>) -> Result<Device> {
    if let Some(needle) = device_name {
        host.input_devices()
            .wrap_err("listing input devices")?
            .find(|device| device.name().is_ok_and(|name| name == needle))
            .ok_or(eyre!("no input device with a name '{needle}'"))
    } else {
        host.default_input_device()
            .ok_or(eyre!("no default input device available"))
    }
}
