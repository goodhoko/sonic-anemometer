use audio_anemometer::computer::Computer;
use criterion::{criterion_group, criterion_main, Criterion};

const MAX_EXPECTED_DELAY_SAMPLES: usize = 2048;
const COMPARISON_WINDOW_WIDTH: usize = 1024;

pub fn single_sample_loopback_and_delay(c: &mut Criterion) {
    c.bench_function("single sample loopback and delay", |b| {
        let mut computer = setup_computer(MAX_EXPECTED_DELAY_SAMPLES, COMPARISON_WINDOW_WIDTH);

        b.iter(|| {
            let sample = computer.output_sample();
            computer.record_sample(sample);
            computer.delay();
        })
    });
}

pub fn single_sample_loopback(c: &mut Criterion) {
    c.bench_function("single sample loopback", |b| {
        let mut computer = setup_computer(MAX_EXPECTED_DELAY_SAMPLES, COMPARISON_WINDOW_WIDTH);
        b.iter(|| {
            let sample = computer.output_sample();
            computer.record_sample(sample);
        })
    });
}

pub fn delay(c: &mut Criterion) {
    c.bench_function("delay", |b| {
        let computer = setup_computer(MAX_EXPECTED_DELAY_SAMPLES, COMPARISON_WINDOW_WIDTH);

        b.iter(|| {
            computer.delay();
        })
    });
}

/// Construct SimpleComputer and run it until its internal buffers are fully populated.
fn setup_computer(
    maximum_expected_delay_samples: usize,
    comparison_window_width: usize,
) -> Computer {
    let mut computer = Computer::new(maximum_expected_delay_samples, comparison_window_width);
    for _ in 0..(maximum_expected_delay_samples + comparison_window_width) {
        let sample = computer.output_sample();
        computer.record_sample(sample);
    }

    computer
}

criterion_group!(
    benches,
    single_sample_loopback,
    single_sample_loopback_and_delay,
    delay,
);
criterion_main!(benches);
