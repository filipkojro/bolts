use std::{
    fs::File,
    io::{Read, Seek},
};

use crate::models::{
    ColumnSpec, GeneralMetaData, Row, RowValue, bin_file_path, idx_file_path, table_file_path,
};

const CHUNK_SIZE: usize = 256 * 1024;

pub fn get_metadata(final_file: String) -> GeneralMetaData {
    let mut file = File::open(final_file + "/meta.json").expect("failed to open metadata file");

    let mut metadata_json = String::new();

    file.read_to_string(&mut metadata_json)
        .expect("failed to read metadata file");

    let general_metadata: GeneralMetaData =
        serde_json::from_str(&metadata_json).expect("failed to parse metadata contents");

    return general_metadata;
}

pub fn read_table_bin_raw_row(
    buf_reader_table_file: &mut File,
    metadata: &GeneralMetaData,
    index: u64,
    raw_row: &mut [u8],
) {
    let number_of_columns = metadata.column_meta_data.len();
    let offset: u64 = index * number_of_columns as u64 * 8;

    buf_reader_table_file
        .seek(std::io::SeekFrom::Start(offset))
        .expect("failed to seek to specified row in table.bin");
    buf_reader_table_file
        .read_exact(raw_row)
        .expect("reading raw row from table.bin");
}

pub fn raw_row_to_row(raw_row: &Vec<u8>, metadata: &GeneralMetaData) -> Row {
    let mut row: Row = Row::with_capacity(metadata.column_meta_data.len());

    for (column_metadata, value_bytes) in metadata
        .column_meta_data
        .iter()
        .zip(raw_row.chunks_exact(8))
    {
        let value = match column_metadata.column_spec {
            ColumnSpec::Long { .. } => RowValue::Long {
                value: i64::from_le_bytes(value_bytes.try_into().unwrap()),
            },
            ColumnSpec::Double { .. } => RowValue::Double {
                value: f64::from_le_bytes(value_bytes.try_into().unwrap()),
            },
        };

        row.push(value);
    }

    return row;
}

pub fn column_eq(file_name: String, column_name: String, value: RowValue) -> Vec<Row> {
    let metadata = get_metadata(file_name.clone());
    let total_bytes = metadata.number_of_rows * 8;
    let mut bytes_processed = 0;

    let mut val_file =
        File::open(bin_file_path(&file_name, &column_name, 0)).expect("failed to open file");

    let mut idx_file =
        File::open(idx_file_path(&file_name, &column_name, 0)).expect("failed to open file");

    let mut table_file = File::open(table_file_path(&file_name)).expect("failed to open table.bin");

    let mut val_chunk = vec![0u8; CHUNK_SIZE];
    let mut idx_chunk = vec![0u8; CHUNK_SIZE];

    let mut raw_row: Vec<u8> = vec![0u8; metadata.column_meta_data.len() * 8];

    let mut rows: Vec<Row> = Vec::new();

    while bytes_processed < total_bytes {
        let bytes_to_read =
            std::cmp::min(CHUNK_SIZE as u64, total_bytes - bytes_processed) as usize;

        val_file
            .read_exact(&mut val_chunk[..bytes_to_read])
            .expect("failed to read val chunk");
        idx_file
            .read_exact(&mut idx_chunk[..bytes_to_read])
            .expect("failed to read idx chunk");

        for (val_bytes, idx_bytes) in val_chunk[..bytes_to_read]
            .chunks_exact(8)
            .zip(idx_chunk[..bytes_to_read].chunks_exact(8))
        {
            let idx: u64 = u64::from_le_bytes(idx_bytes.try_into().unwrap());

            match value {
                RowValue::Long { value } => {
                    let val: i64 = i64::from_le_bytes(val_bytes.try_into().unwrap());
                    if val == value {
                        read_table_bin_raw_row(&mut table_file, &metadata, idx, &mut raw_row);
                        let row = raw_row_to_row(&raw_row, &metadata);
                        rows.push(row);
                    }
                }
                RowValue::Double { value } => {
                    let val: f64 = f64::from_le_bytes(val_bytes.try_into().unwrap());
                    if val == value {
                        read_table_bin_raw_row(&mut table_file, &metadata, idx, &mut raw_row);
                        let row = raw_row_to_row(&raw_row, &metadata);
                        rows.push(row);
                    }
                }
            }
        }

        bytes_processed += bytes_to_read as u64;
    }

    rows

    // for column_metadata in metadata.column_meta_data {
    //     if column_metadata.name == column_name {
    //         match column_metadata.column_spec {
    //             ColumnSpec::Long {
    //                 min_value,
    //                 max_value,
    //                 bin_splits,
    //             } => {
    //                 if value < min_value || value > max_value {
    //                     return;
    //                 }
    //                 let bin_index = bin_splits
    //                     .partition_point(|&x| value >= x)
    //                     .saturating_sub(1);

    //                 let bin_reader =
    //                     File::open(bin_file_path(&file_name.clone(), &column_name, bin_index))
    //                         .expect("failed to open file");
    //             }
    //             ColumnSpec::Double {
    //                 min_value,
    //                 max_value,
    //                 bin_splits,
    //             } => todo!(),
    //         }
    //     }
    // }
}
