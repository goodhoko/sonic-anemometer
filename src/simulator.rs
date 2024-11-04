use rand::random;

use crate::{RingBuffer, Sample};

pub trait Simulator {
    fn play_sample(&mut self, sample: Sample);
    fn acquire_sample(&mut self) -> Sample;
}

pub struct DelaySimulator {
    buffer: RingBuffer<Sample>,
}

impl DelaySimulator {
    pub fn new(delay_samples: usize) -> Self {
        Self {
            buffer: RingBuffer::new(delay_samples),
        }
    }
}

impl Simulator for DelaySimulator {
    fn play_sample(&mut self, sample: Sample) {
        self.buffer.push_back(sample);
    }

    fn acquire_sample(&mut self) -> Sample {
        if self.buffer.is_full() {
            *self
                .buffer
                .iter()
                .next()
                .expect("we just checked it's full")
        } else {
            0.0
        }
    }
}

pub struct AttenuationSimulator {
    attenuation: f32,
    sample: Option<Sample>,
}

impl AttenuationSimulator {
    // TODO: use decibels
    pub fn new(attenuation: f32) -> Self {
        Self {
            attenuation,
            sample: None,
        }
    }
}

impl Simulator for AttenuationSimulator {
    fn play_sample(&mut self, sample: Sample) {
        self.sample = Some(sample);
    }

    fn acquire_sample(&mut self) -> Sample {
        match self.sample {
            Some(sample) => sample * self.attenuation,
            None => 0.0,
        }
    }
}

pub struct NoiseSimulator {
    signal_to_noise_ratio: f32,
    sample: Option<Sample>,
}

impl NoiseSimulator {
    pub fn new(signal_to_noise_ratio: f32) -> Self {
        Self {
            signal_to_noise_ratio,
            sample: None,
        }
    }
}

impl Simulator for NoiseSimulator {
    fn play_sample(&mut self, sample: Sample) {
        self.sample = Some(sample);
    }

    fn acquire_sample(&mut self) -> Sample {
        match self.sample {
            Some(sample) => {
                // TODO: don't assume the signal was produced with the same random() function with
                // the same range (amplitude)
                let noise = random::<f32>() / self.signal_to_noise_ratio;
                sample + noise
            }
            None => 0.0,
        }
    }
}

pub struct CompositeSimulator {
    attenuation: AttenuationSimulator,
    delay: DelaySimulator,
    noise: NoiseSimulator,
}

impl CompositeSimulator {
    pub fn new(delay_samples: usize, attenuation: f32, signal_to_noise_ratio: f32) -> Self {
        Self {
            attenuation: AttenuationSimulator::new(attenuation),
            delay: DelaySimulator::new(delay_samples),
            noise: NoiseSimulator::new(signal_to_noise_ratio),
        }
    }
}

impl Simulator for CompositeSimulator {
    fn play_sample(&mut self, sample: Sample) {
        self.delay.play_sample(sample);

        let sample = self.delay.acquire_sample();
        self.attenuation.play_sample(sample);

        let sample = self.attenuation.acquire_sample();
        self.noise.play_sample(sample);
    }

    fn acquire_sample(&mut self) -> Sample {
        self.noise.acquire_sample()
    }
}
