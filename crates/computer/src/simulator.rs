use rand::random;

use crate::{ring_buffer::RingBuffer, Sample};

pub struct Simulator {
    delay_buffer: Option<RingBuffer<Sample>>,
    attenuation: f32,
    signal_to_noise_ratio: f32,
}

impl Simulator {
    pub fn new(delay_samples: usize, attenuation: f32, signal_to_noise_ratio: f32) -> Self {
        let delay_buffer = if delay_samples > 0 {
            Some(RingBuffer::new(delay_samples))
        } else {
            None
        };

        Self {
            delay_buffer,
            attenuation,
            signal_to_noise_ratio,
        }
    }

    pub fn tick(&mut self, input: Sample) -> Sample {
        let Some(buffer) = self.delay_buffer.as_mut() else {
            // Delay is zero, let the sample just pass through.
            return input;
        };

        // Simulate silence while the buffer is filling up (and returning None).
        let output = buffer.push_back(input).unwrap_or(0.0);

        let noise = random::<f32>() / self.signal_to_noise_ratio;

        output * self.attenuation + noise
    }
}
