use std::f32::consts::PI;

use rand::random;

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
        // TODO: calculate what std would produce RMS equivalent to using uniform distribution
        // between [-1, 1] we used before.
        // Gaussian distribution always produces some samples outside any range.
        // Clamp the few outliers produced from 0.5 STD.
        let sample = Self::random_number_with_gaussian_distribution(0.5, 0.0).clamp(-1.0, 1.0);
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

    // TODO: check correctness
    fn random_number_with_gaussian_distribution(standard_deviation: f32, mean: f32) -> f32 {
        let r1 = random::<f32>();
        let r2 = random::<f32>();
        let n = (-2.0 * r1.ln()).sqrt() * (2.0 * PI * r2).cos();

        n * standard_deviation + mean
    }
}
