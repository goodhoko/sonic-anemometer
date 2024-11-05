use std::ops::Sub;

use rand::random;

use crate::{ring_buffer::RingBuffer, Sample};

pub trait Computer {
    fn output_sample(&mut self) -> Sample;
    fn record_sample(&mut self, sample: Sample);
    fn delay(&self) -> Option<usize>;
}

pub struct SimpleComputer {
    output: RingBuffer<Sample>,
    input: RingBuffer<Sample>,
}

impl SimpleComputer {
    pub fn new(maximum_expected_delay_samples: usize, comparison_window_width: usize) -> Self {
        Self {
            output: RingBuffer::new(maximum_expected_delay_samples + comparison_window_width),
            input: RingBuffer::new(comparison_window_width),
        }
    }

    fn random_sample(&mut self) -> Sample {
        random()
    }
}

impl Computer for SimpleComputer {
    fn output_sample(&mut self) -> Sample {
        let sample = self.random_sample();
        self.output.push_back(sample);
        sample
    }

    fn record_sample(&mut self, sample: Sample) {
        self.input.push_back(sample);
    }

    fn delay(&self) -> Option<usize> {
        if !self.input.is_full() {
            // We haven't yet accumulated enough input samples. We'll need to wait bit more.
            return None;
        }

        let maximum_shift = self
            .output
            .len()
            .checked_sub(self.input.len())
            .expect("we can't have less output samples than input ones");

        // Find the phase shift that produced the minimum compound error.
        // TODO: make this code nicer. Unfortunately f32 isn't Ord so we can't use Iterator::min().
        let mut min_error = f32::MAX;
        let mut corresponding_phase_shift = 0;

        for phase_shift_samples in 0..maximum_shift {
            let output_window = self.output.iter().skip(phase_shift_samples);
            let input_window = self.input.iter();

            // assert!(output_window.clone().count() >= input_window.clone().count());

            let error = output_window
                .zip(input_window)
                .fold(0.0, |acc, (output_sample, input_sample)| {
                    acc + output_sample.sub(input_sample).abs()
                });

            if error < min_error {
                min_error = error;
                corresponding_phase_shift = phase_shift_samples;
            }
        }

        Some(maximum_shift - corresponding_phase_shift)
    }
}
