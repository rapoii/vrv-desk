//! Dedicated Remote File Transfer Manager Engine
//! Handles directory listing, chunked read/write streaming, creation, and deletion.

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FsEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FsListResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub id: String,
    pub path: String,
    pub entries: Vec<FsEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FsReadResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub id: String,
    pub path: String,
    pub offset: u64,
    pub total_size: u64,
    pub eof: bool,
    pub data_b64: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FsWriteResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub id: String,
    pub path: String,
    pub bytes_written: usize,
    pub eof: bool,
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FsActionResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub id: String,
    pub action: String,
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FsErrorResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub id: String,
    pub error: String,
}

// ---------------------------------------------------------------------------
// Zero-dependency RFC 4648 Base64 Implementation
// ---------------------------------------------------------------------------

const BASE64_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        out.push(BASE64_ALPHABET[(b0 >> 2) as usize] as char);
        out.push(BASE64_ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

        if chunk.len() > 1 {
            out.push(BASE64_ALPHABET[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }

        if chunk.len() > 2 {
            out.push(BASE64_ALPHABET[(b2 & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let clean: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if clean.len() % 4 != 0 {
        return Err("Invalid base64 length".to_string());
    }

    let decode_char = |c: u8| -> Result<u8, String> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => Err(format!("Invalid base64 character: {}", c as char)),
        }
    };

    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for chunk in clean.chunks(4) {
        let d0 = decode_char(chunk[0])?;
        let d1 = decode_char(chunk[1])?;
        let d2 = decode_char(chunk[2])?;
        let d3 = decode_char(chunk[3])?;

        out.push((d0 << 2) | (d1 >> 4));
        if chunk[2] != b'=' {
            out.push(((d1 & 0x0F) << 4) | (d2 >> 2));
        }
        if chunk[3] != b'=' {
            out.push(((d2 & 0x03) << 6) | d3);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// File Manager Operations
// ---------------------------------------------------------------------------

pub struct FileManager;

impl FileManager {
    /// Lists a remote directory, or returns system root drives / shortcuts if path is empty.
    pub fn list_directory(raw_path: &str) -> Result<(String, Vec<FsEntry>), String> {
        let trimmed = raw_path.trim();

        // If empty or "roots", discover drives / system root
        if trimmed.is_empty() || trimmed == "roots" || trimmed == "/" && cfg!(windows) {
            return Self::list_roots();
        }

        let path = Path::new(trimmed);
        if !path.exists() {
            return Err(format!("Path not found: {}", trimmed));
        }
        if !path.is_dir() {
            return Err(format!("Path is not a directory: {}", trimmed));
        }

        let canonical_str = path.canonicalize()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| trimmed.to_string())
            .trim_start_matches(r"\\?\")
            .to_string();

        let mut entries = Vec::new();
        let read_dir = fs::read_dir(path).map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in read_dir.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let metadata = entry.metadata().ok();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = if is_dir { 0 } else { metadata.as_ref().map(|m| m.len()).unwrap_or(0) };
            let modified_ms = metadata.and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            entries.push(FsEntry {
                name: file_name,
                is_dir,
                size,
                modified_ms,
            });
        }

        // Sort: directories first, then alphabetical
        entries.sort_by(|a, b| {
            if a.is_dir != b.is_dir {
                b.is_dir.cmp(&a.is_dir)
            } else {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        });

        Ok((canonical_str, entries))
    }

    /// Lists drives and common user folders.
    pub fn list_roots() -> Result<(String, Vec<FsEntry>), String> {
        let mut entries = Vec::new();

        #[cfg(windows)]
        {
            // Discover logical drives A-Z
            for letter in b'A'..=b'Z' {
                let drive_str = format!("{}:\\", letter as char);
                if Path::new(&drive_str).exists() {
                    entries.push(FsEntry {
                        name: drive_str,
                        is_dir: true,
                        size: 0,
                        modified_ms: 0,
                    });
                }
            }
        }

        #[cfg(not(windows))]
        {
            entries.push(FsEntry {
                name: "/".to_string(),
                is_dir: true,
                size: 0,
                modified_ms: 0,
            });
        }

        // Add user profile standard paths if available
        if let Ok(user_profile) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            let p = Path::new(&user_profile);
            for folder in &["Desktop", "Documents", "Downloads", "Pictures", "Music", "Videos"] {
                let sub = p.join(folder);
                if sub.exists() {
                    entries.push(FsEntry {
                        name: sub.to_string_lossy().to_string(),
                        is_dir: true,
                        size: 0,
                        modified_ms: 0,
                    });
                }
            }
        }

        Ok(("Roots".to_string(), entries))
    }

    /// Reads a single chunk of a file at given byte offset.
    pub fn read_chunk(path_str: &str, offset: u64, max_length: usize) -> Result<FsReadResponse, String> {
        let path = Path::new(path_str);
        if !path.exists() {
            return Err(format!("File not found: {}", path_str));
        }

        let mut file = File::open(path).map_err(|e| format!("Cannot open file: {}", e))?;
        let total_size = file.metadata().map_err(|e| e.to_string())?.len();

        file.seek(SeekFrom::Start(offset)).map_err(|e| format!("Seek failed: {}", e))?;

        let chunk_size = max_length.min(65536);
        let mut buf = vec![0u8; chunk_size];
        let bytes_read = file.read(&mut buf).map_err(|e| format!("Read failed: {}", e))?;
        buf.truncate(bytes_read);

        let eof = (offset + bytes_read as u64) >= total_size;
        let data_b64 = base64_encode(&buf);

        Ok(FsReadResponse {
            msg_type: "fs_read_resp".to_string(),
            id: String::new(),
            path: path_str.to_string(),
            offset,
            total_size,
            eof,
            data_b64,
        })
    }

    /// Writes a chunk of data into a target file at given offset.
    pub fn write_chunk(path_str: &str, offset: u64, data_b64: &str, eof: bool) -> Result<FsWriteResponse, String> {
        let data = base64_decode(data_b64)?;
        let path = Path::new(path_str);

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut file = if offset == 0 {
            File::create(path).map_err(|e| format!("Failed to create file: {}", e))?
        } else {
            OpenOptions::new()
                .write(true)
                .open(path)
                .map_err(|e| format!("Failed to open file for append: {}", e))?
        };

        file.seek(SeekFrom::Start(offset)).map_err(|e| format!("Seek failed: {}", e))?;
        file.write_all(&data).map_err(|e| format!("Write failed: {}", e))?;
        file.flush().map_err(|e| format!("Flush failed: {}", e))?;

        Ok(FsWriteResponse {
            msg_type: "fs_write_resp".to_string(),
            id: String::new(),
            path: path_str.to_string(),
            bytes_written: data.len(),
            eof,
            success: true,
        })
    }

    /// Creates a directory at given path.
    pub fn create_dir(path_str: &str) -> Result<(), String> {
        fs::create_dir_all(path_str).map_err(|e| format!("Failed to create directory: {}", e))
    }

    /// Deletes a file or directory.
    pub fn delete_item(path_str: &str, is_dir: bool) -> Result<(), String> {
        let path = Path::new(path_str);
        if !path.exists() {
            return Err(format!("Item does not exist: {}", path_str));
        }

        if is_dir {
            fs::remove_dir_all(path).map_err(|e| format!("Failed to delete directory: {}", e))
        } else {
            fs::remove_file(path).map_err(|e| format!("Failed to delete file: {}", e))
        }
    }
}
