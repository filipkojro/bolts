use std::{
    fs::File,
    io::{BufReader, Read, Seek},
};

use crate::models::GeneralMetaData;

pub fn read_table_bin_row(final_file: String, metadata: GeneralMetaData, index: u64) -> String {
    let number_of_columns = metadata.column_meta_data.len();
    let offset: u64 = index * number_of_columns as u64 * 8;

    let file = File::open(final_file + "/table.bin").expect("failed to open table.bin");

    let mut buf_reader = BufReader::new(file);
    let mut raw_row = vec![0u8; number_of_columns * 8];
    buf_reader
        .seek(std::io::SeekFrom::Start(offset))
        .expect("failed to seek to specified row in table.bin");
    buf_reader
        .read_exact(&mut raw_row)
        .expect("reading raw row from table.bin");

    let mut string_row = String::new();

    for (raw_value, column_metadata) in raw_row.chunks_exact(8).zip(metadata.column_meta_data) {
        match column_metadata.column_spec {
            crate::models::ColumnSpec::Long { .. } => {
                let value: i64 = i64::from_le_bytes(raw_value.try_into().unwrap());
                string_row.push_str(format!("{value} ").as_str());
            }
            crate::models::ColumnSpec::Double { .. } => {
                let value: f64 = f64::from_le_bytes(raw_value.try_into().unwrap());
                string_row.push_str(format!("{value} ").as_str());
            }
        }
    }

    return string_row;
}
