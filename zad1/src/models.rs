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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ColumnMetaData {
    pub name: String,
    pub description: String,
    #[serde(flatten)]
    pub column_spec: ColumnSpec,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralMetaData {
    pub number_of_rows: usize,
    pub column_meta_data: Vec<ColumnMetaData>,
}
