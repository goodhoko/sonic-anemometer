use std::{
    sync::{Arc, RwLock},
    thread,
};

use audio_anemometer::{
    computer::Computer, gui::run_gui, io::run_real_world_audio, simulator::simulate_audio_pipeline,
    tui::run_tui,
};
use clap::Parser;
use color_eyre::eyre::Result;

/// Width (in samples) of the window to use when correlating input signal with the output signal.
const COMPARISON_WINDOW_WIDTH: usize = 1024;
/// This controls how long into history of the played output we look to find just received input.
/// If the actual delay is longer than this we won't be able to measure it.
/// Used as a cap for compute and memory usage.
const MAX_EXPECTED_DELAY_SAMPLES: usize = 2048;

/// By how many samples the simulator delays the produced input (as if coming from microphone)
/// compared to the output (as if fed to speakers).
pub const SIMULATED_DELAY_SAMPLES: usize = 139;
/// How much does the simulator attenuates the signal. (applied as a multiplier to every sample)
const SIMULATED_GAIN: f32 = 1.0;
/// Signal to noise ratio of the simulated physical system.
const SIMULATED_SNR: f32 = 5.0;

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
    Simulate,
    Run {
        #[arg(long, short)]
        input_device: Option<String>,
        #[arg(long, short)]
        output_device: Option<String>,
    },
}

#[derive(Debug, Clone, clap::Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
    #[arg(long)]
    run_gui: bool,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let computer = Arc::new(RwLock::new(Computer::new(
        MAX_EXPECTED_DELAY_SAMPLES,
        COMPARISON_WINDOW_WIDTH,
    )));

    let simulator = matches!(args.command, Command::Simulate).then(|| {
        simulate_audio_pipeline(
            Arc::clone(&computer),
            SIMULATED_DELAY_SAMPLES,
            SIMULATED_GAIN,
            SIMULATED_SNR,
        )
    });

    // We can't collapse this into a single `match` with the above because we need to keep
    // _streams alive and running.
    let _streams = if let Command::Run {
        input_device,
        output_device,
    } = args.command
    {
        Some(run_real_world_audio(
            Arc::clone(&computer),
            input_device,
            output_device,
        )?)
    } else {
        None
    };

    if args.run_gui {
        let c = Arc::clone(&computer);
        thread::spawn(|| {
            run_tui(c);
        });

        // Gui must run on the main thread.
        run_gui(computer, simulator)
    } else {
        run_tui(computer)
    }
}
