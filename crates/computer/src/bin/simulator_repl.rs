use std::io::{self, Write};

use audio_anemometer::{simulator::Simulator, Sample};

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    delay_samples: usize,
    #[arg(short, long, default_value_t = 1.0)]
    gain: f32,
    #[arg(short, long, default_value_t = f32::MAX)]
    signal_to_noise_ratio: f32,
}

fn main() {
    let args = Args::parse();

    let mut simulator = Simulator::new(args.delay_samples, args.gain, args.signal_to_noise_ratio);

    let stdin = io::stdin();
    let mut line = String::new();

    loop {
        print!("< ");
        io::stdout().flush().expect("can flush stdout");
        line.clear();
        stdin.read_line(&mut line).expect("can read line");
        let sample = line.trim().parse::<Sample>().expect("valid number");
        let response = simulator.tick(sample);
        println!("> {response:.0}\n");
    }
}
