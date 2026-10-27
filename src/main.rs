use std::time::{Duration, Instant};

use bolts::{models::RowValue, reader::column_eq};

fn main() {
    const ITERATIONS: u32 = 10;

    let read = column_eq(
        "large-file.dat.bolts".to_string(),
        "c4".to_string(),
        RowValue::Long { value: 14 },
    );
    println!("read: {:?}", read[0]);

    let start = Instant::now();

    for _ in 0..ITERATIONS {
        std::hint::black_box(column_eq(
            "large-file.dat.bolts".to_string(),
            "c4".to_string(),
            RowValue::Long { value: 14 },
        ));
    }

    let total_duration: Duration = start.elapsed();
    let average_duration: Duration = total_duration / ITERATIONS;

    println!("Total time ({} runs): {:?}", ITERATIONS, total_duration);
    println!("Average time per run: {:?}", average_duration);
}
