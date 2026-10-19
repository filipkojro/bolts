use std::{
    fs::File,
    io::Read,
    time::{Duration, Instant},
};

use bolts::{
    models::{bin_file_path, idx_file_path},
    reader::{get_metadata, read_table_bin_raw_row},
};

const CHUNK_SIZE: usize = 256 * 1024;

fn read_equal(column_name: String, eq: i64) -> u64 {
    let mut counter: u64 = 0;
    let bolts_file_name: String = "large-file.dat.bolts".to_string();

    let metadata = get_metadata(bolts_file_name.clone());
    let total_bytes = metadata.number_of_rows * 8;
    let mut bytes_processed = 0;

    let mut val_f =
        File::open(bin_file_path(&bolts_file_name, &column_name, 0)).expect("failed to open file");

    let mut idx_f =
        File::open(idx_file_path(&bolts_file_name, &column_name, 0)).expect("failed to open file");

    let mut table_file =
        File::open(bolts_file_name + "/table.bin").expect("failed to open table.bin");

    let mut val_chunk = vec![0u8; CHUNK_SIZE];
    let mut idx_chunk = vec![0u8; CHUNK_SIZE];

    let mut raw_row: Vec<u8> = vec![0u8; metadata.column_meta_data.len() * 8];

    while bytes_processed < total_bytes {
        let bytes_to_read =
            std::cmp::min(CHUNK_SIZE as u64, total_bytes - bytes_processed) as usize;

        val_f
            .read_exact(&mut val_chunk[..bytes_to_read])
            .expect("failed to read val chunk");
        idx_f
            .read_exact(&mut idx_chunk[..bytes_to_read])
            .expect("failed to read idx chunk");

        for (val_bytes, idx_bytes) in val_chunk[..bytes_to_read]
            .chunks_exact(8)
            .zip(idx_chunk[..bytes_to_read].chunks_exact(8))
        {
            let value: i64 = i64::from_le_bytes(val_bytes.try_into().unwrap());
            let idx: u64 = u64::from_le_bytes(idx_bytes.try_into().unwrap());

            if value == eq {
                read_table_bin_raw_row(&mut table_file, &metadata, idx, &mut raw_row);
                counter += 1;
            }
        }

        bytes_processed += bytes_to_read as u64;
    }

    return counter;
}

fn main() {
    const ITERATIONS: u32 = 1000;

    // Optional warm-up run to prime OS file caches
    let read = read_equal("c4".to_string(), 14);
    println!("read: {}", read);

    let start = Instant::now();

    for _ in 0..ITERATIONS {
        // std::hint::black_box prevents the compiler from optimizing the call away
        std::hint::black_box(read_equal("c4".to_string(), 14));
    }

    let total_duration: Duration = start.elapsed();
    let average_duration: Duration = total_duration / ITERATIONS;

    println!("Total time ({} runs): {:?}", ITERATIONS, total_duration);
    println!("Average time per run: {:?}", average_duration);
}
