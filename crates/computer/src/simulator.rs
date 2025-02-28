use std::{
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use rand::random;

use crate::{computer::Computer, ring_buffer::RingBuffer, Sample};

#[derive(Debug)]
pub struct Simulator {
    delay_buffer: Option<RingBuffer<Sample>>,
    pub gain: f32,
    pub signal_to_noise_ratio: f32,
}

impl Simulator {
    pub fn new(delay_samples: usize, gain: f32, signal_to_noise_ratio: f32) -> Self {
        let delay_buffer = if delay_samples > 0 {
            Some(RingBuffer::new(delay_samples))
        } else {
            None
        };

        Self {
            delay_buffer,
            gain,
            signal_to_noise_ratio,
        }
    }

    pub fn tick(&mut self, input: Sample) -> Sample {
        let output = match self.delay_buffer.as_mut() {
            // Delay is zero, let the sample just pass through.
            None => input,
            // Simulate silence while the buffer is filling up (and returning None).
            Some(buffer) => buffer.push_back(input).unwrap_or(0.0),
        };

        let noise = random::<f32>() / self.signal_to_noise_ratio;

        output * self.gain + noise
    }

    pub fn delay_samples(&self) -> usize {
        self.delay_buffer
            .as_ref()
            .map_or(0, |buffer| buffer.capacity())
    }

    pub fn set_delay(&mut self, delay_samples: usize) {
        match (&mut self.delay_buffer, delay_samples) {
            (None, 0) => {}
            (None, _) => self.delay_buffer = Some(RingBuffer::new(delay_samples)),
            (Some(_), 0) => self.delay_buffer = None,
            (Some(buffer), _) => buffer.set_capacity(delay_samples),
        }
    }
}

/// Spawn a thread that advances the simulator and the computer.
pub fn simulate_audio_pipeline(
    computer: Arc<RwLock<Computer>>,
    delay_samples: usize,
    gain: f32,
    signal_to_noise_ratio: f32,
) -> Arc<RwLock<Simulator>> {
    let simulator = Arc::new(RwLock::new(Simulator::new(
        delay_samples,
        gain,
        signal_to_noise_ratio,
    )));

    {
        let simulator = Arc::clone(&simulator);
        thread::spawn(move || {
            let mut samples = 0;
            let mut last_report = Instant::now();
            loop {
                // TODO: maybe better to lock the computer only once?
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

    simulator
}
