use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Write},
};

#[derive(Debug, Serialize, Deserialize)]
enum ColumnType {
    LONG,
    DOUBLE,
}

#[derive(Debug, Serialize, Deserialize)]
struct OriginalColumnMetaData {
    name: String,
    description: String,
    #[serde(rename = "type")]
    column_type: ColumnType,
}

fn parse_original_column_metadata(file: String) -> Vec<OriginalColumnMetaData> {
    let f = File::open(file).expect("failed to open file");
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ColumnSpec {
    #[serde(rename = "LONG")]
    Long { min_value: i64, max_value: i64 },
    #[serde(rename = "DOUBLE")]
    Double { min_value: f64, max_value: f64 },
}

#[derive(Debug, Serialize, Deserialize)]
struct ColumnMetaData {
    name: String,
    description: String,
    #[serde(flatten)]
    spec: ColumnSpec,
    bin_number: usize,
}

fn save_metadata(file: String, metadata: Vec<ColumnMetaData>) {
    let mut metadata_file =
        File::create(file + "/meta.json").expect("failed to create metadata file");

    let metadata_json = serde_json::to_string(&metadata).expect("failed to parse metadata");

    metadata_file
        .write_all(metadata_json.as_bytes())
        .expect("filed to write into metadata file");
}

fn convert_dat_file(file: String) {
    let original_column_metadata = parse_original_column_metadata(file.clone());

    let final_file_name = format!("{}.bolt", file); // bin optimized large tables

    fs::create_dir_all(final_file_name.clone()).expect("failed to create directory for files");

    let mut meta: Vec<ColumnMetaData> = Vec::new();

    meta.push(ColumnMetaData {
        name: "nammme".into(),
        description: "opnes".into(),
        spec: ColumnSpec::Long {
            min_value: 12,
            max_value: 22,
        },
        bin_number: 22,
    });
    save_metadata(final_file_name, meta);
}

fn main() {
    let input_file_name = "small-file.dat";

    convert_dat_file(input_file_name.into());
}
