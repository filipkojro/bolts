# bin-organized large table storage

Bolts is a storage format with a reader and writer written in Rust.
It's designed for filtering multi-terabyte tables on hard drives and works best when only a small fraction of rows match, and those matches are spread throughout the table.

## How the data is stored

Each column is split into bins with non-overlapping value ranges.
A bin stores each column value alongside its original row index, in ascending row-index order.
A separate file stores the full table in its original row order, so the reader can use an index to jump directly to a full row.

This layout uses up to 3x the space of the raw binary table, excluding metadata and filesystem overhead.
Each value is stored once in the full table and once in its column bin, alongside an equally sized row index.
The extra storage speeds up filtering by letting the reader scan only the relevant bins and fetch full rows where it finds matches.

## Filtering steps

1. Use the filter to choose which bins to scan. For `column1 = 1`, only the bin whose range contains `1` can have matches.
2. Scan the selected bin for matching values and collect their row indexes. The indexes are already in ascending order within the bin.
3. Use those indexes to fetch the full rows. Nearby matches can be fetched together in a larger read to reduce disk seeks.
