use axum::extract::Multipart;

// #[tokio::test]
// async fn upload() {
//     let multipart = Multipart::
//     while let Some(field) = multipart.next_field().await.unwrap() {
//         let filename = if let Some(filename) = field.file_name() {
//             filename.to_string()
//         } else {
//             continue;
//         };
//         let mut buf = field.bytes().await.unwrap().as_mut();
//         let file_id = Uuid::new_v4();
//         write(format!("./store/{file_id}.png"), buf).await.unwrap();
//
//     }
// }
