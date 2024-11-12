use std::{
    ops::Deref,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use audio_anemometer::{
    computer::{Computer, SimpleComputer},
    simulator::Simulator,
};

const COMPARISON_WINDOW_WIDTH: usize = 1024;
const MAX_EXPECTED_DELAY_SAMPLES: usize = 2048;

// TODO: make these dynamically changeable by the user by winit key events
const DELAY_SAMPLES: usize = 999;
const ATTENUATION: f32 = 0.5;
const SIGNAL_TO_NOISE_RATIO: f32 = 5.0;

fn main() {
    let simulator = Arc::new(RwLock::new(Simulator::new(
        DELAY_SAMPLES,
        ATTENUATION,
        SIGNAL_TO_NOISE_RATIO,
    )));
    let computer = Arc::new(RwLock::new(SimpleComputer::new(
        MAX_EXPECTED_DELAY_SAMPLES,
        COMPARISON_WINDOW_WIDTH,
    )));

    spawn_audio_pipeline_simulation(&computer, &simulator);
    // TODO: run GUI instead
    run_tui(computer);
}

/// Spawn a thread that advances the simulator and the computer.
fn spawn_audio_pipeline_simulation(
    computer: &Arc<RwLock<SimpleComputer>>,
    simulator: &Arc<RwLock<Simulator>>,
) {
    let computer = Arc::clone(computer);
    let simulator = Arc::clone(simulator);

    thread::spawn(move || {
        let mut samples = 0;
        let mut last_report = Instant::now();
        loop {
            let output_sample = computer.write().unwrap().output_sample();
            let input_sample = simulator.write().unwrap().tick(output_sample);
            computer.write().unwrap().record_sample(input_sample);

            samples += 1;

            if last_report.elapsed() > Duration::from_secs(1) {
                println!("processed {samples} samples");
                samples = 0;
                last_report = Instant::now();
            }
        }
    });
}

fn run_tui(computer: Arc<RwLock<SimpleComputer>>) {
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
