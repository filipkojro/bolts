use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufRead, BufReader},
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

fn main() {
    let input_file_name = "small-file.dat";

    let original_column_metadata = parse_original_column_metadata(input_file_name.into());

    for column in original_column_metadata {
        println!("{:?}", column);
    }
}
