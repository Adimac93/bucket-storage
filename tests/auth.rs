use sqlx::{PgPool, query};
use tracing::debug;
use tracing_test::traced_test;
use bucket_storage::auth::ArgonHash;

mod tools;
use crate::tools::AppData;



#[traced_test]
#[sqlx::test(fixtures("buckets","bucket_keys"))]
async fn auth(pool: PgPool) {
    let a = query!(r#"
    SELECT * FROM bucket_keys
    "#).fetch_all(&pool).await.unwrap();
    println!("{a:?}");
    let data = AppData::new(pool).await;
    let res = data.client()
        .get(data.api("/download"))
        .basic_auth("195ea586-110f-454a-a7e6-87bbec64c41c",Some("ee014d6f-5798-44b0-9186-f68f3261146e"))
        .send().await.unwrap();
    let body = res.text().await.unwrap();
    debug!("{body}");
}

#[traced_test]
#[sqlx::test(fixtures("buckets","bucket_keys"))]
async fn a(pool: PgPool) {
    let a = query!(r#"
    SELECT * FROM bucket_keys
    "#).fetch_all(&pool).await.unwrap();
    println!("{a:?}");
}

// #[traced_test]
// #[sqlx::test(fixtures("buckets","bucket_keys"))]
// async fn a(pool: PgPool) {
//     let h = ArgonHash::new("ee014d6f-5798-44b0-9186-f68f3261146e".into()).hash().unwrap();
//     debug!("{h}");
// }