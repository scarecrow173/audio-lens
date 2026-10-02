//! Chunked columnar table store.
//!
//! Layout under one table root:
//!
//! ```text
//! schema.json          column names + types
//! index.json           row count + rows per chunk
//! <column>/NNNNNN.bin  one file per column per chunk
//! ```
//!
//! Each chunk file starts with a NULL bitmap (`ceil(rows/8)` bytes,
//! LSB-first per row) followed by the non-null values: `i64`/`f64` as
//! little-endian fixed-width, strings as `u32` little-endian length +
//! UTF-8 payload. Chunking bounds read I/O: a row range touches only
//! the chunks covering it, and only for the selected columns.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use al_job::fsutil::write_durable;

/// Default rows per column chunk.
pub const DEFAULT_CHUNK_ROWS: u64 = 65_536;

/// Supported column element types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    /// 64-bit signed integer.
    I64,
    /// 64-bit IEEE float.
    F64,
    /// UTF-8 string.
    Utf8,
}

/// One column of a table schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Column {
    /// Column name; also used as the chunk directory name.
    pub name: String,
    /// Element type of non-null values.
    pub data_type: DataType,
}

/// One cell value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Cell {
    /// Integer cell.
    I64(i64),
    /// Float cell.
    F64(f64),
    /// String cell.
    Str(String),
    /// NULL cell.
    Null,
}

#[derive(Serialize, Deserialize)]
struct TableIndex {
    rows_per_chunk: u64,
    row_count: u64,
}

/// Appends rows to a table under `root` in uniform-size chunks.
pub struct TableWriter {
    root: PathBuf,
    schema: Vec<Column>,
    rows_per_chunk: u64,
    buffered: Vec<Vec<Cell>>,
    row_count: u64,
}

impl TableWriter {
    /// Create a new table root. Fails if `root` already exists.
    pub fn create(root: &Path, schema: Vec<Column>, rows_per_chunk: u64) -> io::Result<Self> {
        if schema.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "table needs a column",
            ));
        }
        if rows_per_chunk == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "rows_per_chunk must be > 0",
            ));
        }
        validate_schema(&schema)?;
        fs::create_dir(root).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("create table root {}: {e}", root.display()),
            )
        })?;
        let w = Self {
            root: root.to_path_buf(),
            schema,
            rows_per_chunk,
            buffered: Vec::new(),
            row_count: 0,
        };
        // `index.json` is the commit marker: it is written only by
        // `finish`, so a half-written table fails `TableReader::open`
        // instead of reading as an empty-but-valid table.
        let schema_json = serde_json::to_vec_pretty(&w.schema)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        write_durable(&w.root.join("schema.json"), &schema_json)?;
        Ok(w)
    }

    /// Append one row. The row length and non-null cell types must
    /// match the schema.
    pub fn push(&mut self, row: &[Cell]) -> io::Result<()> {
        if row.len() != self.schema.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "row has {} cells, schema has {}",
                    row.len(),
                    self.schema.len()
                ),
            ));
        }
        for (cell, col) in row.iter().zip(&self.schema) {
            let ok = matches!(
                (cell, col.data_type),
                (Cell::I64(_), DataType::I64)
                    | (Cell::F64(_), DataType::F64)
                    | (Cell::Str(_), DataType::Utf8)
                    | (Cell::Null, _)
            );
            if !ok {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("cell type does not match column {:?}", col),
                ));
            }
        }
        self.buffered.push(row.to_vec());
        if self.buffered.len() as u64 == self.rows_per_chunk {
            self.flush_chunk()?;
        }
        Ok(())
    }

    /// Flush the partial tail chunk and write the index. Must be
    /// called before readers open the table.
    pub fn finish(mut self) -> io::Result<()> {
        self.flush_chunk()?;
        let index = TableIndex {
            rows_per_chunk: self.rows_per_chunk,
            row_count: self.row_count,
        };
        let index_json = serde_json::to_vec_pretty(&index)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        write_durable(&self.root.join("index.json"), &index_json)
    }

    fn flush_chunk(&mut self) -> io::Result<()> {
        if self.buffered.is_empty() {
            return Ok(());
        }
        let chunk_idx = self.row_count / self.rows_per_chunk;
        let n = self.buffered.len();
        for (col_idx, col) in self.schema.iter().enumerate() {
            let mut bitmap = vec![0u8; n.div_ceil(8)];
            let mut payload = Vec::new();
            for (row_idx, row) in self.buffered.iter().enumerate() {
                match &row[col_idx] {
                    Cell::Null => bitmap[row_idx / 8] |= 1 << (row_idx % 8),
                    Cell::I64(v) => payload.extend_from_slice(&v.to_le_bytes()),
                    Cell::F64(v) => payload.extend_from_slice(&v.to_le_bytes()),
                    Cell::Str(s) => {
                        let len = u32::try_from(s.len()).map_err(|_| {
                            io::Error::new(
                                io::ErrorKind::InvalidInput,
                                format!("string cell exceeds u32 length: {}", s.len()),
                            )
                        })?;
                        payload.extend_from_slice(&len.to_le_bytes());
                        payload.extend_from_slice(s.as_bytes());
                    }
                }
            }
            let mut bytes = bitmap;
            bytes.extend_from_slice(&payload);
            let dir = self.root.join(&col.name);
            fs::create_dir_all(&dir)?;
            write_durable(&dir.join(format!("{chunk_idx:06}.bin")), &bytes)?;
        }
        self.row_count += n as u64;
        self.buffered.clear();
        Ok(())
    }
}

/// Column names become directory names: reject anything that could
/// escape the table root or alias another column.
fn validate_schema(schema: &[Column]) -> io::Result<()> {
    let mut seen = std::collections::HashSet::new();
    for col in schema {
        let name = &col.name;
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.contains('/')
            || name.contains('\\')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsafe column name {name:?}"),
            ));
        }
        if !seen.insert(name.as_str()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("duplicate column name {name:?}"),
            ));
        }
    }
    Ok(())
}

/// Reads row ranges and column subsets from a table root.
pub struct TableReader {
    root: PathBuf,
    schema: Vec<Column>,
    index: TableIndex,
}

impl TableReader {
    /// Open a finished table.
    pub fn open(root: &Path) -> io::Result<Self> {
        let schema: Vec<Column> = serde_json::from_slice(&fs::read(root.join("schema.json"))?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let index: TableIndex = serde_json::from_slice(&fs::read(root.join("index.json"))?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        // On-disk schema/index are untrusted: reject names that would
        // escape the table root and a chunk size that would divide by
        // zero.
        validate_schema(&schema)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        if index.rows_per_chunk == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "index.json has rows_per_chunk == 0",
            ));
        }
        Ok(Self {
            root: root.to_path_buf(),
            schema,
            index,
        })
    }

    /// Table schema.
    pub fn schema(&self) -> &[Column] {
        &self.schema
    }

    /// Total row count.
    pub fn row_count(&self) -> u64 {
        self.index.row_count
    }

    /// Read rows `[row_start, row_end)` for the selected columns
    /// (`None` = all columns, in schema order).
    pub fn read_range(
        &self,
        row_start: u64,
        row_end: u64,
        columns: Option<&[usize]>,
    ) -> io::Result<Vec<Vec<Cell>>> {
        if row_end > self.index.row_count || row_start >= row_end {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "row range [{row_start}, {row_end}) out of {}",
                    self.index.row_count
                ),
            ));
        }
        let col_idxs: Vec<usize> = match columns {
            Some(cols) => cols.to_vec(),
            None => (0..self.schema.len()).collect(),
        };
        for &c in &col_idxs {
            if c >= self.schema.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("column index {c} out of {}", self.schema.len()),
                ));
            }
        }
        // Materialize the selected columns for the covered chunks only.
        let mut out: Vec<Vec<Cell>> = vec![Vec::new(); col_idxs.len()];
        let mut chunk = row_start / self.index.rows_per_chunk;
        while chunk * self.index.rows_per_chunk < row_end {
            let chunk_row_start = chunk * self.index.rows_per_chunk;
            let chunk_row_end =
                (chunk_row_start + self.index.rows_per_chunk).min(self.index.row_count);
            for (out_col, &schema_col) in col_idxs.iter().enumerate() {
                let cells = self.read_column_chunk(&self.schema[schema_col], chunk)?;
                for (row_idx, cell) in cells.into_iter().enumerate() {
                    let row = chunk_row_start + row_idx as u64;
                    if row >= row_start && row < row_end {
                        out[out_col].push(cell);
                    }
                }
                let _ = chunk_row_end;
            }
            chunk += 1;
        }
        Ok(out)
    }

    fn read_column_chunk(&self, col: &Column, chunk: u64) -> io::Result<Vec<Cell>> {
        let path = self.root.join(&col.name).join(format!("{chunk:06}.bin"));
        let mut file = File::open(&path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let chunk_row_start = chunk * self.index.rows_per_chunk;
        let n = ((chunk_row_start + self.index.rows_per_chunk).min(self.index.row_count)
            - chunk_row_start) as usize;
        let bitmap_len = n.div_ceil(8);
        if bytes.len() < bitmap_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("chunk {} too short", path.display()),
            ));
        }
        let bitmap = &bytes[..bitmap_len];
        let mut pos = bitmap_len;
        let mut cells = Vec::with_capacity(n);
        for row in 0..n {
            if bitmap[row / 8] & (1 << (row % 8)) != 0 {
                cells.push(Cell::Null);
                continue;
            }
            let cell = match col.data_type {
                DataType::I64 => {
                    let end = pos + 8;
                    let v = i64::from_le_bytes(
                        bytes
                            .get(pos..end)
                            .ok_or_else(|| trunc(&path))?
                            .try_into()
                            .unwrap(),
                    );
                    pos = end;
                    Cell::I64(v)
                }
                DataType::F64 => {
                    let end = pos + 8;
                    let v = f64::from_le_bytes(
                        bytes
                            .get(pos..end)
                            .ok_or_else(|| trunc(&path))?
                            .try_into()
                            .unwrap(),
                    );
                    pos = end;
                    Cell::F64(v)
                }
                DataType::Utf8 => {
                    let len_end = pos + 4;
                    let len = u32::from_le_bytes(
                        bytes
                            .get(pos..len_end)
                            .ok_or_else(|| trunc(&path))?
                            .try_into()
                            .unwrap(),
                    ) as usize;
                    let end = len_end + len;
                    let s =
                        std::str::from_utf8(bytes.get(len_end..end).ok_or_else(|| trunc(&path))?)
                            .map_err(|_| trunc(&path))?;
                    pos = end;
                    Cell::Str(s.to_string())
                }
            };
            cells.push(cell);
        }
        if pos != bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "chunk {} has {} trailing bytes",
                    path.display(),
                    bytes.len() - pos
                ),
            ));
        }
        Ok(cells)
    }
}

fn trunc(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("truncated chunk {}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Vec<Column> {
        vec![
            Column {
                name: "t".into(),
                data_type: DataType::F64,
            },
            Column {
                name: "v".into(),
                data_type: DataType::I64,
            },
            Column {
                name: "label".into(),
                data_type: DataType::Utf8,
            },
        ]
    }

    fn write_table(dir: &Path, chunk_rows: u64, rows: usize) -> PathBuf {
        let root = dir.join("table");
        let mut w = TableWriter::create(&root, schema(), chunk_rows).unwrap();
        for i in 0..rows {
            let label = if i % 3 == 0 {
                Cell::Null
            } else {
                Cell::Str(format!("r{i}"))
            };
            w.push(&[Cell::F64(i as f64 * 0.5), Cell::I64(i as i64), label])
                .unwrap();
        }
        w.finish().unwrap();
        root
    }

    #[test]
    fn round_trip_full_read() {
        let dir = std::env::temp_dir().join(format!("tbl-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        let root = write_table(&dir, 4, 10);
        let r = TableReader::open(&root).unwrap();
        assert_eq!(r.row_count(), 10);
        let cols = r.read_range(0, 10, None).unwrap();
        assert_eq!(cols[0][9], Cell::F64(4.5));
        assert_eq!(cols[1][3], Cell::I64(3));
        assert_eq!(cols[2][0], Cell::Null);
        assert_eq!(cols[2][1], Cell::Str("r1".into()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn range_and_column_selection() {
        let dir = std::env::temp_dir().join(format!("tbl-r-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        let root = write_table(&dir, 4, 10);
        let r = TableReader::open(&root).unwrap();
        // Rows 3..8 cross the chunk boundary at row 4.
        let cols = r.read_range(3, 8, Some(&[2])).unwrap();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].len(), 5);
        assert_eq!(cols[0][0], Cell::Null); // row 3
        assert_eq!(cols[0][1], Cell::Str("r4".into()));
        assert!(r.read_range(9, 11, None).is_err());
        assert!(r.read_range(0, 2, Some(&[7])).is_err());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_mismatched_row() {
        let dir = std::env::temp_dir().join(format!("tbl-b-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        let mut w = TableWriter::create(&dir.join("t"), schema(), 4).unwrap();
        assert!(w.push(&[Cell::I64(1)]).is_err());
        assert!(
            w.push(&[Cell::Str("x".into()), Cell::I64(1), Cell::Null])
                .is_err()
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unsafe_and_duplicate_column_names_rejected() {
        let dir = std::env::temp_dir().join(format!("tbl-n-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        for bad in ["..", "a/b", "a\\b", ""] {
            let schema = vec![Column {
                name: bad.into(),
                data_type: DataType::F64,
            }];
            assert!(TableWriter::create(&dir.join("t1"), schema, 4).is_err());
            fs::remove_dir_all(dir.join("t1")).ok();
        }
        let dup = vec![
            Column {
                name: "v".into(),
                data_type: DataType::F64,
            },
            Column {
                name: "v".into(),
                data_type: DataType::I64,
            },
        ];
        assert!(TableWriter::create(&dir.join("t2"), dup, 4).is_err());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unfinished_table_is_not_readable() {
        let dir = std::env::temp_dir().join(format!("tbl-u-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        let root = dir.join("t");
        {
            let mut w = TableWriter::create(&root, schema(), 4).unwrap();
            for i in 0..8 {
                w.push(&[Cell::F64(i as f64), Cell::I64(i), Cell::Null])
                    .unwrap();
            }
            // Dropped without finish(): index.json is never written.
        }
        assert!(TableReader::open(&root).is_err());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupt_index_fails_not_panics() {
        let dir = std::env::temp_dir().join(format!("tbl-i-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        let root = write_table(&dir, 4, 10);
        fs::write(
            root.join("index.json"),
            r#"{"rows_per_chunk":0,"row_count":10}"#,
        )
        .unwrap();
        assert!(TableReader::open(&root).is_err());
        fs::remove_dir_all(&dir).ok();
    }
}
