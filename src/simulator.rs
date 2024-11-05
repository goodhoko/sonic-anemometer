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
        let output = match &mut self.delay_buffer.as_mut() {
            None => input,
            Some(buffer) if buffer.is_full() => {
                *buffer.iter().next().expect("we just checked it's full")
            }
            _ => 0.0,
        };

        if let Some(buffer) = self.delay_buffer.as_mut() {
            buffer.push_back(input);
        }

        let noise = random::<f32>() / self.signal_to_noise_ratio;

        output * self.attenuation + noise
    }
}
