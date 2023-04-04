use bucket_storage::auth::ArgonHash;

#[test]
fn hash_1() {
    let hash = ArgonHash::new("abc".into()).hash().unwrap();
    let is_valid = ArgonHash::new("abc".into()).verify(hash).unwrap();
    assert!(is_valid)
}

#[test]
fn hash_2() {
    let hash = ArgonHash::new("ab".into()).hash().unwrap();
    let is_valid = ArgonHash::new("abc".into()).verify(hash).unwrap();
    assert!(!is_valid)
}