use mirror_core::file_manager::{base64_decode, base64_encode, FileManager};
use std::fs;
use std::path::Path;

#[test]
fn test_base64_roundtrip() {
    let test_data = b"Hello, VrV Desk File Transfer! 12345\x00\xff\xfe\xaa";
    let encoded = base64_encode(test_data);
    let decoded = base64_decode(&encoded).expect("Decode should succeed");
    assert_eq!(&decoded[..], &test_data[..]);
}

#[test]
fn test_list_roots_returns_entries() {
    let (name, entries) = FileManager::list_roots().expect("list_roots should succeed");
    assert_eq!(name, "Roots");
    assert!(!entries.is_empty(), "Should discover at least one root drive or user path");
}

#[test]
fn test_file_chunk_write_and_read() {
    let temp_dir = std::env::temp_dir().join("vrv_file_test");
    let _ = fs::create_dir_all(&temp_dir);
    let file_path = temp_dir.join("test_chunk.dat");
    let file_str = file_path.to_string_lossy().to_string();

    let sample_bytes = b"0123456789ABCDEF-VRV-TEST-PAYLOAD";
    let data_b64 = base64_encode(sample_bytes);

    // 1. Write chunk at offset 0
    let write_res = FileManager::write_chunk(&file_str, 0, &data_b64, true)
        .expect("write_chunk should succeed");
    assert!(write_res.success);
    assert_eq!(write_res.bytes_written, sample_bytes.len());

    // 2. Read chunk back
    let read_res = FileManager::read_chunk(&file_str, 0, 1024)
        .expect("read_chunk should succeed");
    assert_eq!(read_res.total_size, sample_bytes.len() as u64);
    assert!(read_res.eof);
    assert_eq!(read_res.data_b64, data_b64);

    let decoded = base64_decode(&read_res.data_b64).expect("valid b64");
    assert_eq!(&decoded[..], &sample_bytes[..]);

    // 3. Delete file
    FileManager::delete_item(&file_str, false).expect("delete_item file should succeed");
    assert!(!file_path.exists());

    // 4. Create and delete dir
    let sub_dir = temp_dir.join("test_sub_dir");
    let sub_str = sub_dir.to_string_lossy().to_string();
    FileManager::create_dir(&sub_str).expect("create_dir should succeed");
    assert!(sub_dir.exists());
    FileManager::delete_item(&sub_str, true).expect("delete_item dir should succeed");
    assert!(!sub_dir.exists());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_list_directory_returns_sorted_items() {
    let temp_dir = std::env::temp_dir().join("vrv_dir_list_test");
    let _ = fs::create_dir_all(&temp_dir);

    // Create a dir and a file
    let sub = temp_dir.join("alpha_folder");
    let _ = fs::create_dir(&sub);
    let f1 = temp_dir.join("beta_file.txt");
    let _ = fs::write(&f1, b"beta");

    let (_path, entries) = FileManager::list_directory(&temp_dir.to_string_lossy())
        .expect("list_directory should succeed");

    assert!(entries.len() >= 2);
    // alpha_folder is a dir, beta_file is not -> dir must come first
    assert!(entries[0].is_dir);
    assert_eq!(entries[0].name, "alpha_folder");

    let _ = fs::remove_dir_all(&temp_dir);
}
