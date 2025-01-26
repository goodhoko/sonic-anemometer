use audio_anemometer::{computer::Computer, gui::run_gui, simulator::simulate_audio_pipeline};
use eyre::Result;
use std::sync::{Arc, RwLock};

/// By how many samples the simulator delays the produced input (as if coming from microphone)
/// compared to the output (as if fed to speakers).
pub const DELAY_SAMPLES: usize = 139;
/// How much does the simulator attenuates the signal. (applied as a multiplier to every sample)
const GAIN: f32 = 1.0;
/// Signal to noise ratio of the simulated physical system.
const SIGNAL_TO_NOISE_RATIO: f32 = 5.0;

/// How wide a window to use when searching for the input signal in the output.
const COMPARISON_WINDOW_WIDTH: usize = 1024;
/// For the purpose of the simulation we know this is in fact exactly DELAY_SAMPLES.
/// In reality though we'll use some heuristic to estimate this based on the physical setup
/// as well as the delay intrinsic to the digital part of the pipeline.
/// This controls how long into history of the played output we look to find just received input.
/// Used as a cap for compute and memory usage.
const MAX_EXPECTED_DELAY_SAMPLES: usize = 2048;

fn main() -> Result<()> {
    color_eyre::install()?;

    let computer = Arc::new(RwLock::new(Computer::new(
        MAX_EXPECTED_DELAY_SAMPLES,
        COMPARISON_WINDOW_WIDTH,
    )));

    let simulator = simulate_audio_pipeline(
        Arc::clone(&computer),
        DELAY_SAMPLES,
        GAIN,
        SIGNAL_TO_NOISE_RATIO,
    );

    // TODO: optionally (based on arguments) run tui instead.
    run_gui(computer, Some(simulator));

    Ok(())
}
