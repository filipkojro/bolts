use std::{
    fs::File,
    io::{Error, ErrorKind, Read, Result, Seek},
};

use crate::models::{
    ColumnMetaData, ColumnSpec, GeneralMetaData, Row, RowValue, bin_file_path, idx_file_path,
    table_file_path,
};

const CHUNK_SIZE: usize = 256 * 1024;

pub fn get_metadata(final_file: String) -> Result<GeneralMetaData> {
    let mut file = File::open(final_file + "/meta.json")?;

    let mut metadata_json = String::new();

    file.read_to_string(&mut metadata_json)?;

    let general_metadata: GeneralMetaData = serde_json::from_str(&metadata_json)?;

    Ok(general_metadata)
}

fn get_column_metadata_by_name(
    metadata: GeneralMetaData,
    column_name: String,
) -> Result<ColumnMetaData> {
    for column_metadata in metadata.column_meta_data {
        if column_metadata.name == column_name {
            return Ok(column_metadata);
        }
    }
    return Err(Error::new(ErrorKind::InvalidInput, "invalid column_name"));
}

pub fn read_table_bin_raw_row(
    buf_reader_table_file: &mut File,
    metadata: &GeneralMetaData,
    index: u64,
    raw_row: &mut [u8],
) -> Result<()> {
    let number_of_columns = metadata.column_meta_data.len();
    let offset: u64 = index * number_of_columns as u64 * 8;

    buf_reader_table_file.seek(std::io::SeekFrom::Start(offset))?;
    buf_reader_table_file.read_exact(raw_row)?;
    Ok(())
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

pub fn column_eq(file_name: String, column_name: String, value_eq: RowValue) -> Result<Vec<Row>> {
    let mut rows: Vec<Row> = Vec::new();

    let metadata = get_metadata(file_name.clone())?;
    let column_metadata = get_column_metadata_by_name(metadata.clone(), column_name.clone())?;

    let bin: usize; // there is only one possible bin for equal

    match column_metadata.column_spec {
        ColumnSpec::Long {
            min_value,
            max_value,
            bin_splits,
        } => match value_eq {
            RowValue::Long { value } => {
                if value < min_value || max_value < value {
                    return Ok(rows);
                }
                bin = bin_splits
                    .partition_point(|&x| x <= value)
                    .saturating_sub(1);
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "type of value_eq is different than column",
                ));
            }
        },
        ColumnSpec::Double {
            min_value,
            max_value,
            bin_splits,
        } => match value_eq {
            RowValue::Double { value } => {
                if value < min_value || max_value < value {
                    return Ok(rows);
                }
                bin = bin_splits
                    .partition_point(|&x| x <= value)
                    .saturating_sub(1);
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "type of value_eq is different than column",
                ));
            }
        },
    }

    let total_bytes = metadata.number_of_rows * 8;
    let mut bytes_processed = 0;

    let mut val_file = File::open(bin_file_path(&file_name, &column_name, bin))?;

    let mut idx_file = File::open(idx_file_path(&file_name, &column_name, bin))?;

    let mut table_file = File::open(table_file_path(&file_name))?;

    let mut val_chunk = vec![0u8; CHUNK_SIZE];
    let mut idx_chunk = vec![0u8; CHUNK_SIZE];

    let mut raw_row: Vec<u8> = vec![0u8; metadata.column_meta_data.len() * 8];

    while bytes_processed < total_bytes {
        let bytes_to_read =
            std::cmp::min(CHUNK_SIZE as u64, total_bytes - bytes_processed) as usize;

        val_file.read_exact(&mut val_chunk[..bytes_to_read])?;
        idx_file.read_exact(&mut idx_chunk[..bytes_to_read])?;

        for (val_bytes, idx_bytes) in val_chunk[..bytes_to_read]
            .chunks_exact(8)
            .zip(idx_chunk[..bytes_to_read].chunks_exact(8))
        {
            let idx: u64 = u64::from_le_bytes(idx_bytes.try_into().unwrap());

            match value_eq {
                RowValue::Long { value } => {
                    let val: i64 = i64::from_le_bytes(val_bytes.try_into().unwrap());
                    if val == value {
                        read_table_bin_raw_row(&mut table_file, &metadata, idx, &mut raw_row)?;
                        let row = raw_row_to_row(&raw_row, &metadata);
                        rows.push(row);
                    }
                }
                RowValue::Double { value } => {
                    let val: f64 = f64::from_le_bytes(val_bytes.try_into().unwrap());
                    if val == value {
                        read_table_bin_raw_row(&mut table_file, &metadata, idx, &mut raw_row)?;
                        let row = raw_row_to_row(&raw_row, &metadata);
                        rows.push(row);
                    }
                }
            }
        }

        bytes_processed += bytes_to_read as u64;
    }

    Ok(rows)
}
