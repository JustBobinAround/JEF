
use super::jef::indexer::{
    last_chars_until_forward_slash,
    starts_with_prefix_simd,
    get_hash,
}; 
use super::jef::opener::{
    Config,
};
#[test]
fn test_last_chars_until_forward_slash() {
    // Test cases

    // Test case 1: Forward slash found at the end
    let result = last_chars_until_forward_slash("apple/banana/orange/grape");
    assert_eq!(result, "grape");

    let result = last_chars_until_forward_slash("./src/main.rs");
    assert_eq!(result, "main.rs");

    // Test case 2: Forward slash found in the middle
    let result = last_chars_until_forward_slash("apple/banana/orange");
    assert_eq!(result, "orange");

    // Test case 3: Forward slash not found
    let result = last_chars_until_forward_slash("apple-banana-orange");
    assert_eq!(result, "apple-banana-orange");

    // Test case 4: Edge case with empty string
    let result = last_chars_until_forward_slash("");
    assert_eq!(result, "");

    // Test case 5: Edge case with only a forward slash
    let result = last_chars_until_forward_slash("/");
    assert_eq!(result, "");
}
#[test]
fn test_starts_with_prefix_simd() {
    // Test cases

    let result = starts_with_prefix_simd("apple/banana/orange","apple");
    assert_eq!(result, true);

    let result = starts_with_prefix_simd("apple-banana-orange","banana");
    assert_eq!(result, false);
}

#[test]
fn test_config(){
    let result = Config::default_config();
    println!("{:?}", result);
}
