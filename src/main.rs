use std::process::Command;
use std::time::{Duration, Instant};

use bolts::reader::column_eq_p;
use bolts::writer::convert_dat_file;
use bolts::{models::RowValue, reader::column_eq};

fn drop_os_caches() {
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("sync").status();

        let status = Command::new("sh")
            .arg("-c")
            .arg("echo 3 > /proc/sys/vm/drop_caches")
            .status()
            .expect("Failed to execute cache drop command");

        assert!(
            status.success(),
            "Failed to drop caches. Ensure you are running as root."
        );
    }

    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("sync").status();
        let status = Command::new("sudo")
            .arg("purge")
            .status()
            .expect("Failed to execute purge command");

        assert!(
            status.success(),
            "Failed to drop caches. macOS requires sudo for purge."
        );
    }
}

fn main() {
    // let start = Instant::now();
    // convert_dat_file("large-file.dat".to_string()).expect("aaaaa");
    // println!("finished in : {:?}", start.elapsed());

    const ITERATIONS: u32 = 5;

    drop_os_caches();

    // Warm-up or initial validation read (optional)
    let read = column_eq(
        "large-file.dat.bolts".to_string(),
        "c4".to_string(),
        RowValue::Long { value: 14 },
    )
    .unwrap();
    println!("read: {}", read.len());

    let mut total_duration = Duration::new(0, 0);

    for i in 0..ITERATIONS {
        // 1. Clear the cache BEFORE starting the timer
        drop_os_caches();

        // 2. Start the timer for this specific iteration
        let start = Instant::now();

        std::hint::black_box(
            column_eq_p(
                "large-file.dat.bolts".to_string(),
                "c4".to_string(),
                RowValue::Long { value: 14 },
            )
            .unwrap(),
        );

        // 3. Accumulate the duration
        let iteration_time = start.elapsed();
        total_duration += iteration_time;

        println!("Run {}: {:?}", i + 1, iteration_time);
    }

    let average_duration: Duration = total_duration / ITERATIONS;

    println!(
        "Total execution time ({} runs): {:?}",
        ITERATIONS, total_duration
    );
    println!("Average time per run (cold cache): {:?}", average_duration);
}
