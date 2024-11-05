use rand::random;

use crate::{ring_buffer::RingBuffer, Sample};

pub struct Simulator {
    delay_buffer: RingBuffer<Sample>,
    attenuation: f32,
    signal_to_noise_ratio: f32,
}

impl Simulator {
    pub fn new(delay_samples: usize, attenuation: f32, signal_to_noise_ratio: f32) -> Self {
        Self {
            delay_buffer: RingBuffer::new(delay_samples),
            attenuation,
            signal_to_noise_ratio,
        }
    }

    pub fn tick(&mut self, sample: Sample) -> Sample {
        let output = if self.delay_buffer.is_full() {
            *self
                .delay_buffer
                .iter()
                .next()
                .expect("we just checked it's full")
        } else {
            0.0
        };

        self.delay_buffer.push_back(sample);

        let noise = random::<f32>() / self.signal_to_noise_ratio;

        output * self.attenuation + noise
    }
}
