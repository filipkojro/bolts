use std::{
    fs::File,
    io::{BufReader, Read, Seek},
};

use crate::models::{ColumnSpec, GeneralMetaData, bin_file_path};

pub fn get_metadata(final_file: String) -> GeneralMetaData {
    let mut file = File::open(final_file + "/meta.json").expect("failed to open metadata file");

    let mut metadata_json = String::new();

    file.read_to_string(&mut metadata_json)
        .expect("failed to read metadata file");

    let general_metadata: GeneralMetaData =
        serde_json::from_str(&metadata_json).expect("failed to parse metadata contents");

    return general_metadata;
}

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

pub fn find_column_equals<T>(file_name: String, column_name: String, value: T)
where
    T: PartialOrd<i64>,
    T: PartialOrd<f64>,
{
    let metadata = get_metadata(file_name.clone());

    for column_metadata in metadata.column_meta_data {
        if column_metadata.name == column_name {
            match column_metadata.column_spec {
                ColumnSpec::Long {
                    min_value,
                    max_value,
                    bin_splits,
                } => {
                    if value < min_value || value > max_value {
                        return;
                    }
                    let bin_index = bin_splits
                        .partition_point(|&x| value >= x)
                        .saturating_sub(1);

                    let bin_reader = BufReader::new(
                        File::open(bin_file_path(&file_name.clone(), &column_name, bin_index))
                            .expect("failed to open file"),
                    );
                }
                ColumnSpec::Double {
                    min_value,
                    max_value,
                    bin_splits,
                } => todo!(),
            }
        }
    }
}
