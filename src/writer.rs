use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Result, Write},
};

use crate::{
    models::{
        BinFiles, ColumnMetaData, ColumnSpec, ColumnType, GeneralMetaData, OriginalColumnMetaData,
        bin_file_path, idx_file_path,
    },
    reader::get_metadata,
};

fn save_metadata(final_file: String, metadata: GeneralMetaData) -> Result<()> {
    let mut metadata_file = File::create(final_file + "/meta.json")?;

    let metadata_json = serde_json::to_string(&metadata)?;

    metadata_file.write_all(metadata_json.as_bytes())?;

    Ok(())
}

fn parse_original_column_metadata(original_file: String) -> Result<Vec<OriginalColumnMetaData>> {
    let f = File::open(original_file)?;
    let reader = BufReader::new(f);

    let mut original_column_metadata_raw: String = String::new();

    for line in reader.lines() {
        let line_raw = line.unwrap();

        if line_raw.starts_with("#") {
            original_column_metadata_raw.push_str(&line_raw.strip_prefix("#").unwrap().trim());
        } else {
            break;
        }
    }

    let original_column_metadata: Vec<OriginalColumnMetaData> =
        serde_json::from_str(&original_column_metadata_raw)
            .expect("failed to parse original column metadata");
    return Ok(original_column_metadata);
}

fn generate_metadata(original_file: String, final_file: String) -> Result<()> {
    let original_column_metadata = parse_original_column_metadata(original_file.clone())?;

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

    let mut number_of_rows: u64 = 0;

    let f = File::open(original_file)?;
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

    const NUMBER_OF_BINS: usize = 1;

    for column_metadata in new_column_metadata.iter_mut() {
        match &mut column_metadata.column_spec {
            ColumnSpec::Long {
                min_value,
                max_value,
                bin_splits,
            } => {
                if (NUMBER_OF_BINS as i64) < *max_value - *min_value {
                    let bin_size = (*max_value - *min_value) / NUMBER_OF_BINS as i64;
                    for bin_num in 0..NUMBER_OF_BINS {
                        bin_splits.push(bin_size * bin_num as i64 + *min_value);
                    }
                } else {
                    for bin_num in 0..(*max_value - *min_value) {
                        bin_splits.push(bin_num + *min_value);
                    }
                }
            }
            ColumnSpec::Double {
                min_value,
                max_value,
                bin_splits,
            } => {
                let bin_size = (*max_value - *min_value) / NUMBER_OF_BINS as f64;
                for bin_num in 0..NUMBER_OF_BINS {
                    bin_splits.push(bin_size * bin_num as f64 + *min_value);
                }
            }
        }
    }

    save_metadata(
        final_file,
        GeneralMetaData {
            number_of_rows: number_of_rows,
            column_meta_data: new_column_metadata,
        },
    )?;

    Ok(())
}

fn create_column_bins<T>(
    final_file: &String,
    column_metadata: &ColumnMetaData,
    bins: &Vec<T>,
) -> Result<()> {
    fs::create_dir_all(format!("{}/{}", final_file.clone(), column_metadata.name))?;

    for i in 0..bins.len() {
        let _c = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(bin_file_path(final_file, &column_metadata.name, i))?;
        let _i = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(idx_file_path(final_file, &column_metadata.name, i))?;
    }

    return Ok(());
}

fn put_into_bin_i64(
    index: u64,
    value: i64,
    bin_splits: &Vec<i64>,
    final_file: &String,
    column_metadata: &ColumnMetaData,
) -> Result<()> {
    let bin_index = bin_splits
        .partition_point(|&x| x <= value)
        .saturating_sub(1);

    let mut bin_files: BinFiles = BinFiles {
        column: OpenOptions::new().append(true).open(bin_file_path(
            final_file,
            &column_metadata.name,
            bin_index,
        ))?,
        indexes: OpenOptions::new().append(true).open(idx_file_path(
            final_file,
            &column_metadata.name,
            bin_index,
        ))?,
    };

    let value_bytes = value.to_le_bytes();
    bin_files.column.write(&value_bytes)?;

    let index_bytes = index.to_le_bytes();
    bin_files.indexes.write(&index_bytes)?;

    Ok(())
}
fn put_into_bin_f64(
    index: u64,
    value: f64,
    bin_splits: &Vec<f64>,
    final_file: &String,
    column_metadata: &ColumnMetaData,
) -> Result<()> {
    let bin_index = bin_splits
        .partition_point(|&x| x <= value)
        .saturating_sub(1);

    let mut bin_files: BinFiles = BinFiles {
        column: OpenOptions::new().append(true).open(bin_file_path(
            final_file,
            &column_metadata.name,
            bin_index,
        ))?,
        indexes: OpenOptions::new().append(true).open(idx_file_path(
            final_file,
            &column_metadata.name,
            bin_index,
        ))?,
    };

    let value_bytes = value.to_le_bytes();
    bin_files.column.write(&value_bytes)?;

    let index_bytes = index.to_le_bytes();
    bin_files.indexes.write(&index_bytes)?;

    Ok(())
}

fn convert_data(original_file: String, final_file: String) -> Result<()> {
    let metadata = get_metadata(final_file.clone())?;

    let f = File::open(original_file)?;
    let reader = BufReader::new(f);

    let table_rows_per_flush: usize = 1000000 / metadata.column_meta_data.len() / 8;

    let mut bin_table_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(final_file.clone() + "/table.bin")?;

    for column_metadata in metadata.column_meta_data.iter() {
        match &column_metadata.column_spec {
            ColumnSpec::Long { bin_splits, .. } => {
                create_column_bins(&final_file, &column_metadata, &bin_splits)?;
            }
            ColumnSpec::Double { bin_splits, .. } => {
                create_column_bins(&final_file, &column_metadata, &bin_splits)?;
            }
        }
    }

    let table_buffer_size = table_rows_per_flush * metadata.column_meta_data.len() * 8;
    let mut table_buffer: Vec<u8> = Vec::with_capacity(table_buffer_size);

    let mut row_index: u64 = 0;

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

                        put_into_bin_i64(
                            row_index as u64,
                            value,
                            bin_splits,
                            &final_file,
                            &metadata.column_meta_data[column_index],
                        )?;
                    }
                    ColumnSpec::Double { bin_splits, .. } => {
                        let value: f64 = value_string
                            .parse()
                            .expect("failed to parse string into i64");
                        let value_bytes = value.to_le_bytes();
                        table_buffer.extend_from_slice(&value_bytes);

                        put_into_bin_f64(
                            row_index as u64,
                            value,
                            bin_splits,
                            &final_file,
                            &metadata.column_meta_data[column_index],
                        )?;
                    }
                }
            }

            row_index += 1;

            if table_buffer.len() >= table_buffer_size {
                bin_table_file.write_all(table_buffer.as_slice())?;

                table_buffer.clear();
            }
        }
    }
    if table_buffer.len() > 0 {
        bin_table_file.write_all(table_buffer.as_slice())?;

        table_buffer.clear();
    }
    Ok(())
}

pub fn convert_dat_file(original_file: String) -> Result<()> {
    let final_file_name = format!("{original_file}.bolts"); // bin organized large table storage

    fs::create_dir_all(final_file_name.clone())?;

    generate_metadata(original_file.clone(), final_file_name.clone())?;

    convert_data(original_file, final_file_name)?;

    Ok(())
}
