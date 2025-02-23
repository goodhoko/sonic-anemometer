use rand::distributions::Distribution;
use rand::thread_rng;
use statrs::distribution::Normal;

use crate::{ring_buffer::RingBuffer, Sample};

#[derive(Debug, Clone)]
pub struct Computer {
    output: RingBuffer<Sample>,
    input: RingBuffer<Sample>,
}

impl Computer {
    pub fn new(maximum_expected_delay_samples: usize, comparison_window_width: usize) -> Self {
        Self {
            output: RingBuffer::new(maximum_expected_delay_samples + comparison_window_width),
            input: RingBuffer::new(comparison_window_width),
        }
    }

    /// Return the next audio sample in cpal's F32 format.
    pub fn output_sample(&mut self) -> Sample {
        // Generate random noise with normal distribution to approximate real-world noise.
        // Set standard deviation to 0.5 to utilize entire cpal range (-1, 1) without excessive clipping.
        // With STD of 0.5 about 5% of samples end up outside the range and are clamped below.
        let distribution = Normal::new(0.0, 0.5).expect("mean and standard deviation are sane");
        let sample = distribution.sample(&mut thread_rng()).clamp(-1.0, 1.0) as f32;

        self.output.push_back(sample);
        sample
    }

    pub fn record_sample(&mut self, sample: Sample) {
        self.input.push_back(sample);
    }

    pub fn delay(&self) -> Option<usize> {
        if !self.input.is_full() {
            // We haven't yet accumulated enough input samples. We'll need to wait bit more.
            return None;
        }

        // +1 needs to be there to cover 0 delay.
        let maximum_shift = self.output.len().saturating_sub(self.input.len()) + 1;

        // Find the phase shift that produced the minimum compound error.
        // TODO: make this code nicer. Unfortunately f32 isn't Ord so we can't use Iterator::min().
        let mut min_error = f32::MAX;
        let mut corresponding_phase_shift = 0;

        for phase_shift_samples in 0..maximum_shift {
            let output_window = self.output.iter().skip(phase_shift_samples);
            let input_window = self.input.iter();

            let error = output_window
                .zip(input_window)
                .fold(0.0, |acc, (output_sample, input_sample)| {
                    acc + (output_sample - input_sample).abs()
                });

            if error < min_error {
                min_error = error;
                corresponding_phase_shift = phase_shift_samples;
            }
        }

        // Subtract the +1 we added to maximum_shift above.
        Some(maximum_shift - corresponding_phase_shift - 1)
    }

    pub fn input_buffer(&self) -> &RingBuffer<Sample> {
        &self.input
    }

    pub fn output_buffer(&self) -> &RingBuffer<Sample> {
        &self.output
    }
}
