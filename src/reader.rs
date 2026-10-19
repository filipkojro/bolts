use std::{
    fs::File,
    io::{BufReader, Read, Seek},
};

use crate::models::{ColumnSpec, GeneralMetaData, bin_file_path};

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

pub fn find_column_equals<T>(file_name: String, column_name: String, value: T)
where
    T: PartialOrd<i64>,
    T: PartialOrd<f64>,
{
    let metadata = get_metadata(file_name.clone());

    for column_metadata in metadata.column_meta_data {
        if column_metadata.name == column_name {
            match column_metadata.column_spec {
                ColumnSpec::Long {
                    min_value,
                    max_value,
                    bin_splits,
                } => {
                    if value < min_value || value > max_value {
                        return;
                    }
                    let bin_index = bin_splits
                        .partition_point(|&x| value >= x)
                        .saturating_sub(1);

                    let bin_reader = BufReader::new(
                        File::open(bin_file_path(&file_name.clone(), &column_name, bin_index))
                            .expect("failed to open file"),
                    );
                }
                ColumnSpec::Double {
                    min_value,
                    max_value,
                    bin_splits,
                } => todo!(),
            }
        }
    }
}
