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
