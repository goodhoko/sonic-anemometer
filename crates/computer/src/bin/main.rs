use std::{
    sync::{Arc, RwLock},
    thread::spawn,
};

use audio_anemometer::{computer::Computer, gui::run_gui, io::run_real_world_audio, tui::run_tui};
use color_eyre::eyre::Result;

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

    // Keep the streams running by not dropping them.
    let _streams = run_real_world_audio(Arc::clone(&computer))?;

    let c = Arc::clone(&computer);
    spawn(|| {
        run_tui(c);
    });

    run_gui(computer, None)
}
