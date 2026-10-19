use std::{
    fs::File,
    io::{BufReader, Read},
    time::{Duration, Instant},
};

use bolts::{
    models::{bin_file_path, idx_file_path},
    reader::get_metadata,
};

fn count_equal() -> usize {
    let mut f = BufReader::new(
        File::open(bin_file_path(
            &"large-file.dat.bolt".to_string(),
            &"c4".to_string(),
            0,
        ))
        .expect("failed to open file"),
    );

    let mut idx_f = BufReader::new(
        File::open(idx_file_path(
            &"large-file.dat.bolt".to_string(),
            &"c4".to_string(),
            0,
        ))
        .expect("failed to open file"),
    );

    let metadata = get_metadata("large-file.dat.bolt".to_string());

    let mut buffer: Vec<u8> = vec![0u8; metadata.number_of_rows * 8];
    let mut idx_buffer: Vec<u8> = vec![0u8; metadata.number_of_rows * 8];

    f.read_exact(&mut buffer).expect("fail");
    idx_f.read_exact(&mut idx_buffer).expect("fail");

    let mut counter: usize = 0;

    for (value_bytes, idx_bytes) in buffer.chunks_exact(8).zip(idx_buffer.chunks_exact(8)) {
        let value: i64 = i64::from_le_bytes(value_bytes.try_into().unwrap());
        let idx: u64 = u64::from_le_bytes(idx_bytes.try_into().unwrap());

        if value == 14 {
            counter += 1;
        }
    }

    counter
}

fn main() {
    const ITERATIONS: u32 = 1000;

    // Optional warm-up run to prime OS file caches
    let _ = count_equal();

    let start = Instant::now();

    for _ in 0..ITERATIONS {
        // std::hint::black_box prevents the compiler from optimizing the call away
        std::hint::black_box(count_equal());
    }

    let total_duration: Duration = start.elapsed();
    let average_duration: Duration = total_duration / ITERATIONS;

    println!("Total time ({} runs): {:?}", ITERATIONS, total_duration);
    println!("Average time per run: {:?}", average_duration);
}
