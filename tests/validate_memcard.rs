use alfatch_vmc::vmc::vmc_core::validate_mc_file;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_valid_vmc() {
    let mut temp_file = NamedTempFile::new().unwrap();

    let valid_header = "Sony PS2 Memory Card Format       "; //32 bytes size 
    write!(temp_file, "{valid_header}").unwrap();

    let result = validate_mc_file(temp_file.path().to_str().unwrap());

    assert!(result.is_ok());
    assert! {result.unwrap()};
}

#[test]
fn test_invalid_vmc() {
    let mut temp_file = NamedTempFile::new().unwrap();

    let invalid_header = "Invalid PS2 memory card format...";
    write!(temp_file, "{invalid_header}").unwrap();

    let result = validate_mc_file(temp_file.path().to_str().unwrap());

    assert!(result.is_ok());
    assert! {!result.unwrap()};
}

#[test]
fn test_with_non_existent_file() {
    let result = validate_mc_file("void");
    assert!(result.is_err());
}
