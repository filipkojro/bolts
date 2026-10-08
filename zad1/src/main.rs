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
    number_of_bins: usize,
}

fn save_metadata(final_file: String, metadata: Vec<ColumnMetaData>) {
    let mut metadata_file =
        File::create(final_file + "/meta.json").expect("failed to create metadata file");

    let metadata_json = serde_json::to_string(&metadata).expect("failed to parse metadata");

    metadata_file
        .write_all(metadata_json.as_bytes())
        .expect("filed to write into metadata file");
}

fn generate_metadata(original_file: String, final_file: String) {
    let original_column_metadata = parse_original_column_metadata(original_file.clone());

    let mut new_metadata: Vec<ColumnMetaData> = Vec::new();

    for original_column in original_column_metadata {
        let new_metadata_column_spec: ColumnSpec;
        match original_column.column_type {
            ColumnType::DOUBLE => {
                new_metadata_column_spec = ColumnSpec::Double {
                    min_value: f64::MAX,
                    max_value: f64::MIN,
                };
            }
            ColumnType::LONG => {
                new_metadata_column_spec = ColumnSpec::Long {
                    min_value: i64::MAX,
                    max_value: i64::MIN,
                };
            }
        }
        new_metadata.push(ColumnMetaData {
            name: original_column.name,
            description: original_column.description,
            spec: new_metadata_column_spec,
            number_of_bins: 0,
        });
    }

    save_metadata(final_file, new_metadata);
}

fn convert_dat_file(original_file: String) {
    let final_file_name = format!("{}.bolt", original_file); // bin optimized large tables

    fs::create_dir_all(final_file_name.clone()).expect("failed to create directory for files");

    generate_metadata(original_file, final_file_name);
}

fn main() {
    let input_file_name = "small-file.dat";

    convert_dat_file(input_file_name.into());
}
