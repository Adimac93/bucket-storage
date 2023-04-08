use bucket_storage::auth::ArgonHash;

#[test]
fn hash_1() {
    let hash = ArgonHash::hash("abc").unwrap();
    let is_valid = ArgonHash::verify("abc", &hash).unwrap();
    assert!(is_valid)
}

#[test]
fn hash_2() {
    let hash = ArgonHash::hash("ab").unwrap();
    let is_valid = ArgonHash::verify("abc", &hash).unwrap();
    assert!(!is_valid)
}