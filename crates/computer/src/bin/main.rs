use std::{
    ops::Deref,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use audio_anemometer::{
    computer::Computer,
    simulator::{simulate_audio_pipeline, Simulator},
};

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

fn main() {
    let simulator = Arc::new(RwLock::new(Simulator::new(
        DELAY_SAMPLES,
        ATTENUATION,
        SIGNAL_TO_NOISE_RATIO,
    )));
    let computer = Arc::new(RwLock::new(Computer::new(
        MAX_EXPECTED_DELAY_SAMPLES,
        COMPARISON_WINDOW_WIDTH,
    )));

    simulate_audio_pipeline(&computer, &simulator);

    let mut accumulated_delay = 0;
    let mut delays = 0;
    let mut last_report = Instant::now();
    loop {
        // Computing the delay() is much more expensive than cloning the entire computer.
        // To lower lock contention, copy a snapshot of the computer to this thread
        // and immediately release the lock.
        let computer = computer.read().unwrap().deref().clone();

        if let Some(delay) = computer.delay() {
            accumulated_delay += delay;
            delays += 1;

            if last_report.elapsed() > Duration::from_secs(1) {
                let avg = accumulated_delay as f32 / delays as f32;
                println!("delay {avg} samples (averaged over {delays} computations)");
                accumulated_delay = 0;
                delays = 0;
                last_report = Instant::now();
            }
        } else {
            // The computer is not ready yet. Give it some time to accumulate more samples.
            thread::sleep(Duration::from_millis(100));
        }
    }
}
