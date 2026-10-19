use std::{
    fs::File,
    io::{BufReader, Read},
    time::{Duration, Instant},
};

use bolts::{
    models::{bin_file_path, idx_file_path},
    reader::{get_metadata, read_table_bin_raw_row},
};

const CHUNK_SIZE: usize = 256 * 1024;

fn read_equal(eq: i64) {
    let bolts_file_name: String = "large-file.dat.bolts".to_string();

    let mut f = BufReader::new(
        File::open(bin_file_path(&bolts_file_name, &"c4".to_string(), 0))
            .expect("failed to open file"),
    );

    let mut idx_f = BufReader::new(
        File::open(idx_file_path(&bolts_file_name, &"c4".to_string(), 0))
            .expect("failed to open file"),
    );

    let metadata = get_metadata(bolts_file_name.clone());

    let mut buffer: Vec<u8> = vec![0u8; metadata.number_of_rows * 8];
    let mut idx_buffer: Vec<u8> = vec![0u8; metadata.number_of_rows * 8];

    f.read_exact(&mut buffer).expect("fail");
    idx_f.read_exact(&mut idx_buffer).expect("fail");

    let mut table_file =
        File::open(bolts_file_name + "/table.bin").expect("failed to open table.bin");
    // let mut table_buf_reader = BufReader::new(table_file);

    let mut raw_row: Vec<u8> = vec![0u8; metadata.column_meta_data.len() * 8];

    for (value_bytes, idx_bytes) in buffer.chunks_exact(8).zip(idx_buffer.chunks_exact(8)) {
        let value: i64 = i64::from_le_bytes(value_bytes.try_into().unwrap());
        let idx: u64 = u64::from_le_bytes(idx_bytes.try_into().unwrap());

        if value == eq {
            read_table_bin_raw_row(&mut table_file, &metadata, idx, &mut raw_row);
        }
    }
}

fn main() {
    const ITERATIONS: u32 = 10;

    // Optional warm-up run to prime OS file caches
    read_equal(14);

    let start = Instant::now();

    for _ in 0..ITERATIONS {
        // std::hint::black_box prevents the compiler from optimizing the call away
        std::hint::black_box(read_equal(14));
    }

    let total_duration: Duration = start.elapsed();
    let average_duration: Duration = total_duration / ITERATIONS;

    println!("Total time ({} runs): {:?}", ITERATIONS, total_duration);
    println!("Average time per run: {:?}", average_duration);
}
