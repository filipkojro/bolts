use rayon::prelude::*;
use std::io::BufReader;
use std::time::Instant;
use std::{
    fs::File,
    io::{Error, ErrorKind, Read, Result, Seek},
    os::unix::fs::FileExt,
};

use crate::models::{
    ColumnMetaData, ColumnSpec, GeneralMetaData, Row, RowValue, bin_file_path, idx_file_path,
    table_file_path,
};

const CHUNK_SIZE: usize = 256 * 1024;

pub fn get_metadata(final_file: String) -> Result<GeneralMetaData> {
    let file = File::open(final_file + "/meta.json")?;
    let reader = BufReader::new(file);
    let general_metadata: GeneralMetaData = serde_json::from_reader(reader)?;
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

pub fn raw_row_to_row(raw_row: &[u8], metadata: &GeneralMetaData) -> Row {
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

struct CoalescedRange {
    start_offset: u64,
    length: usize,
    target_idx: Vec<u64>,
}

const GAP_THRESHOLD_BYTES: u64 = 128 * 1024;

fn coalesce_idxs(idxs: &[u64], metadata: &GeneralMetaData) -> Vec<CoalescedRange> {
    if idxs.is_empty() {
        return vec![];
    }

    let row_bytes = metadata.column_meta_data.len() as u64 * 8;

    let mut ranges: Vec<CoalescedRange> = Vec::new();
    let mut current_start: u64 = idxs[0] * row_bytes;
    let mut current_end: u64 = current_start + row_bytes;
    let mut current_rows: Vec<u64> = vec![idxs[0]];

    for &idx in &idxs[1..] {
        let row_offset = idx * row_bytes;
        let gap = row_offset - current_end;

        if gap <= GAP_THRESHOLD_BYTES {
            current_end = row_offset + row_bytes;
            current_rows.push(idx);
        } else {
            ranges.push(CoalescedRange {
                start_offset: current_start,
                length: (current_end - current_start) as usize,
                target_idx: current_rows,
            });
            current_start = row_offset;
            current_end = row_offset + row_bytes;
            current_rows = vec![idx];
        }
    }
    ranges.push(CoalescedRange {
        start_offset: current_start,
        length: (current_end - current_start) as usize,
        target_idx: current_rows,
    });

    ranges
}

fn par_read_coalesced(
    file: &File,
    ranges: &[CoalescedRange],
    metadata: &GeneralMetaData,
) -> Vec<Row> {
    let row_bytes = metadata.column_meta_data.len() as u64 * 8;

    ranges
        .par_iter()
        .flat_map(|range| {
            let mut chunk_buf = vec![0u8; range.length];
            file.read_exact_at(&mut chunk_buf, range.start_offset)
                .expect("disk read failed");

            let mut local_rows = Vec::with_capacity(range.target_idx.len());

            for &row_idx in &range.target_idx {
                let row_offset_in_buf = ((row_idx * row_bytes) - range.start_offset) as usize;
                let row_data =
                    &chunk_buf[row_offset_in_buf..row_offset_in_buf + row_bytes as usize];
                local_rows.push(raw_row_to_row(row_data, metadata));
            }

            local_rows
        })
        .collect()
}

pub fn column_eq_p(file_name: String, column_name: String, value_eq: RowValue) -> Result<Vec<Row>> {
    let rows: Vec<Row> = Vec::new();

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

    let mut val_chunk = vec![0u8; CHUNK_SIZE];
    let mut idx_chunk = vec![0u8; CHUNK_SIZE];

    let mut found_idx: Vec<u64> = Vec::with_capacity(CHUNK_SIZE);

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
                        found_idx.push(idx);
                    }
                }
                RowValue::Double { value } => {
                    let val: f64 = f64::from_le_bytes(val_bytes.try_into().unwrap());
                    if val == value {
                        found_idx.push(idx);
                    }
                }
            }
        }

        bytes_processed += bytes_to_read as u64;
    }

    let table_file = File::open(table_file_path(&file_name))?;

    let ranges = coalesce_idxs(&found_idx, &metadata);

    Ok(par_read_coalesced(&table_file, &ranges, &metadata))
}
