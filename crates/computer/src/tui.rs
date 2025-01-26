use std::{
    collections::BTreeMap,
    ops::Deref,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use crate::computer::Computer;

pub fn run_tui(computer: Arc<RwLock<Computer>>) -> ! {
    let mut measurements = Vec::new();
    let mut last_report = Instant::now();
    loop {
        // Computing the delay() is much more expensive than cloning the entire computer.
        // To lower lock contention, copy a snapshot of the computer to this thread
        // and immediately release the lock.
        let computer = computer.read().unwrap().deref().clone();

        if let Some(delay) = computer.delay() {
            measurements.push(delay);

            if last_report.elapsed() > Duration::from_secs(1) {
                let avg = measurements.iter().sum::<usize>() as f64 / measurements.len() as f64;
                measurements.sort();
                let histogram =
                    measurements
                        .iter()
                        .fold(BTreeMap::new(), |mut buckets, measurement| {
                            let bucket = measurement / 100;
                            let entry = buckets.entry(bucket).or_insert(0u32);
                            *entry += 1;
                            buckets
                        });

                println!(
                    "avg: {avg} samples (averaged over {} measurments)",
                    measurements.len()
                );
                println!("histogram: {:#?}", histogram);
                measurements.drain(..);
                last_report = Instant::now();
            }
        } else {
            // The computer is not ready yet. Give it some time to accumulate more samples.
            thread::sleep(Duration::from_millis(100));
        }
    }
}
