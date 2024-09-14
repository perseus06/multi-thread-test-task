// src/utils.rs

use std::io::{Result, Write, Read};

// Split file into smaller chunks
pub fn split_file(file: &[u8]) -> Result<Vec<Vec<u8>>> {
    let chunk_size = 1024 * 1024; // 1MB per chunk
    let mut chunks = vec![];
    let mut cursor = 0;
    while cursor < file.len() {
        let end = std::cmp::min(cursor + chunk_size, file.len());
        chunks.push(file[cursor..end].to_vec());
        cursor = end;
    }
    Ok(chunks)
}

// Merge file chunks back into the original file
pub fn merge_file_chunks(chunks: Vec<Vec<u8>>) -> Vec<u8> {
    let mut merged_file = vec![];
    for chunk in chunks {
        merged_file.extend(chunk);
    }
    merged_file
}
