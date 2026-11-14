use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ColumnType {
    LONG,
    DOUBLE,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OriginalColumnMetaData {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub column_type: ColumnType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ColumnSpec {
    #[serde(rename = "LONG")]
    Long {
        min_value: i64,
        max_value: i64,
        bin_splits: Vec<i64>,
    },
    #[serde(rename = "DOUBLE")]
    Double {
        min_value: f64,
        max_value: f64,
        bin_splits: Vec<f64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMetaData {
    pub name: String,
    pub description: String,
    #[serde(flatten)]
    pub column_spec: ColumnSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralMetaData {
    pub number_of_rows: u64,
    pub column_meta_data: Vec<ColumnMetaData>,
}

pub struct BinBuffers {
    pub column: Vec<u8>,
    pub index: Vec<u8>,
}

pub type AllBinsBuffers = HashMap<String, HashMap<usize, BinBuffers>>;

pub fn bin_file_path(file_name: &String, column_name: &String, bin_number: usize) -> String {
    return format!("{}/{}/{}.bin", file_name, column_name, bin_number);
}
pub fn idx_file_path(file_name: &String, column_name: &String, bin_number: usize) -> String {
    return format!("{}/{}/{}.idx", file_name, column_name, bin_number);
}
pub fn table_file_path(file_name: &String) -> String {
    return format!("{}/table.bin", file_name);
}

#[derive(Debug, Clone)]
pub enum RowValue {
    Long { value: i64 },
    Double { value: f64 },
}

pub type Row = Vec<RowValue>;
