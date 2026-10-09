use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
};

mod models;
mod reader;

use crate::{
    models::{ColumnMetaData, ColumnSpec, ColumnType, GeneralMetaData, OriginalColumnMetaData},
    reader::read_table_bin_row,
};

fn parse_original_column_metadata(original_file: String) -> Vec<OriginalColumnMetaData> {
    let f = File::open(original_file).expect("failed to open file");
    let reader = BufReader::new(f);

    let mut original_column_metadata_raw: String = String::new();

    for line in reader.lines() {
        let line_raw = line.unwrap();

        if line_raw.starts_with("#") {
            original_column_metadata_raw.push_str(
                &line_raw
                    .strip_prefix("#")
                    .expect("failed to strip # prefix")
                    .trim(),
            );
        } else {
            break;
        }
    }

    let original_column_metadata: Vec<OriginalColumnMetaData> =
        serde_json::from_str(&original_column_metadata_raw)
            .expect("failed to parse original column metadata");
    return original_column_metadata;
}

fn save_metadata(final_file: String, metadata: GeneralMetaData) {
    let mut metadata_file =
        File::create(final_file + "/meta.json").expect("failed to create metadata file");

    let metadata_json = serde_json::to_string(&metadata).expect("failed to parse metadata");

    metadata_file
        .write_all(metadata_json.as_bytes())
        .expect("filed to write into metadata file");
}

fn get_metadata(final_file: String) -> GeneralMetaData {
    let mut file = File::open(final_file + "/meta.json").expect("failed to open metadata file");

    let mut metadata_json = String::new();

    file.read_to_string(&mut metadata_json)
        .expect("failed to read metadata file");

    let general_metadata: GeneralMetaData =
        serde_json::from_str(&metadata_json).expect("failed to parse metadata contents");

    return general_metadata;
}

fn generate_metadata(original_file: String, final_file: String) {
    let original_column_metadata = parse_original_column_metadata(original_file.clone());

    let mut new_column_metadata: Vec<ColumnMetaData> = Vec::new();

    for original_column in original_column_metadata {
        let new_metadata_column_spec: ColumnSpec;
        match original_column.column_type {
            ColumnType::DOUBLE => {
                new_metadata_column_spec = ColumnSpec::Double {
                    min_value: f64::MAX,
                    max_value: f64::MIN,
                    bin_splits: Vec::new(),
                };
            }
            ColumnType::LONG => {
                new_metadata_column_spec = ColumnSpec::Long {
                    min_value: i64::MAX,
                    max_value: i64::MIN,
                    bin_splits: Vec::new(),
                };
            }
        }
        new_column_metadata.push(ColumnMetaData {
            name: original_column.name,
            description: original_column.description,
            column_spec: new_metadata_column_spec,
        });
    }

    let mut number_of_rows: usize = 0;

    let f = File::open(original_file).expect("failed to open file");
    let reader = BufReader::new(f);

    for line in reader.lines() {
        let line_raw = line.unwrap();

        if !line_raw.starts_with("#") {
            let row = line_raw.split_ascii_whitespace();

            number_of_rows += 1;

            for (column_index, value_string) in row.enumerate() {
                match &mut new_column_metadata[column_index].column_spec {
                    ColumnSpec::Long {
                        min_value,
                        max_value,
                        ..
                    } => {
                        let value: i64 = value_string
                            .parse()
                            .expect("failed to parse string into i64");

                        if *max_value < value {
                            *max_value = value;
                        }

                        if value < *min_value {
                            *min_value = value;
                        }
                    }
                    ColumnSpec::Double {
                        min_value,
                        max_value,
                        ..
                    } => {
                        let value: f64 = value_string
                            .parse()
                            .expect("failed to parse string into i64");

                        if *max_value < value {
                            *max_value = value;
                        }

                        if value < *min_value {
                            *min_value = value;
                        }
                    }
                }
            }
        }
    }

    for column_metadata in new_column_metadata.iter_mut() {
        match &mut column_metadata.column_spec {
            ColumnSpec::Long {
                min_value,
                max_value,
                bin_splits,
            } => {
                *bin_splits = vec![*min_value, *max_value];
            }
            ColumnSpec::Double {
                min_value,
                max_value,
                bin_splits,
            } => {
                *bin_splits = vec![*min_value, *max_value];
            }
        }
    }

    save_metadata(
        final_file,
        GeneralMetaData {
            number_of_rows: number_of_rows,
            column_meta_data: new_column_metadata,
        },
    );
}

fn convert_data(original_file: String, final_file: String) {
    let metadata = get_metadata(final_file.clone());

    let f = File::open(original_file).expect("failed to open file");
    let reader = BufReader::new(f);

    let table_rows_per_flush: usize = 1000000 / metadata.column_meta_data.len() / 8;

    let mut bin_table_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(final_file + "/table.bin")
        .expect("failed to open table.bin");

    let table_buffer_size = table_rows_per_flush * metadata.column_meta_data.len() * 8;
    let mut table_buffer: Vec<u8> = Vec::with_capacity(table_buffer_size);

    for line in reader.lines() {
        let line_raw = line.unwrap();

        if !line_raw.starts_with("#") {
            let row = line_raw.split_ascii_whitespace();

            for (column_index, value_string) in row.enumerate() {
                match &metadata.column_meta_data[column_index].column_spec {
                    ColumnSpec::Long { bin_splits, .. } => {
                        let value: i64 = value_string
                            .parse()
                            .expect("failed to parse string into i64");
                        let value_bytes = value.to_le_bytes();
                        table_buffer.extend_from_slice(&value_bytes);
                    }
                    ColumnSpec::Double { bin_splits, .. } => {
                        let value: f64 = value_string
                            .parse()
                            .expect("failed to parse string into i64");
                        let value_bytes = value.to_le_bytes();
                        table_buffer.extend_from_slice(&value_bytes);
                    }
                }
            }

            if table_buffer.len() == table_buffer_size {
                bin_table_file
                    .write_all(table_buffer.as_slice())
                    .expect("failed to write table.bin");

                table_buffer.clear();
            }
        }
    }
    if table_buffer.len() > 0 {
        bin_table_file
            .write_all(table_buffer.as_slice())
            .expect("failed to write table.bin");

        table_buffer.clear();
    }
}

fn convert_dat_file(original_file: String) {
    let final_file_name = format!("{original_file}.bolt"); // bin optimized large tables

    fs::create_dir_all(final_file_name.clone()).expect("failed to create directory for files");

    generate_metadata(original_file.clone(), final_file_name.clone());

    convert_data(original_file, final_file_name);
}

fn main() {
    let input_file_name = String::from("small-file.dat");

    convert_dat_file(input_file_name.clone());

    let final_file_name: String = format!("{input_file_name}.bolt");
    let metadata = get_metadata(final_file_name.clone());
    let row = read_table_bin_row(final_file_name, metadata, 1);
    println!("row: {row}");
}
