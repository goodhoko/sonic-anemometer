use std::{thread, time::Duration};

use computer::{Computer, SimpleComputer};
use ring_buffer::RingBuffer;
use simulator::Simulator;

mod computer;
mod ring_buffer;
mod simulator;

/// By how many samples the simulator delays the produced input (as if coming from microphone)
/// compared to the output (as if fed to speakers).
const DELAY_SAMPLES: usize = 23;
/// How much does the simulator attenuates the signal. (applied as a multiplier to every sample)
const ATTENUATION: f32 = 0.5;
/// Signal to noise ratio of the simulated physical system.
const SIGNAL_TO_NOISE_RATIO: f32 = 5.0;

/// How wide a window to use when searching for the input signal in the output.
const COMPARISON_WINDOW_WIDTH: usize = 50;
/// For the purpose of the simulation we know this is in fact exactly DELAY_SAMPLES.
/// In reality though we'll use some heuristic to estimate this based on the physical setup
/// as well as the delay intrinsic to the digital part of the pipeline.
/// This controls how long into history of the played output we look to find just received input.
/// Used as a cap for compute and memory usage.
const MAX_EXPECTED_DELAY_SAMPLES: usize = DELAY_SAMPLES * 2;

type Sample = f32;

fn main() {
    let mut simulator = Simulator::new(DELAY_SAMPLES, ATTENUATION, SIGNAL_TO_NOISE_RATIO);
    let mut computer = SimpleComputer::new(MAX_EXPECTED_DELAY_SAMPLES, COMPARISON_WINDOW_WIDTH);

    loop {
        let output_sample = computer.output_sample();
        let input_sample = simulator.tick(output_sample);
        computer.record_sample(input_sample);

        let delay = computer.delay();
        println!("Delay: {delay:?}");

        thread::sleep(Duration::from_millis(100));
    }
}
