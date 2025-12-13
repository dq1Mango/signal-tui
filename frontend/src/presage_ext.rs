// use presage::{
//   Manager,
//   libsignal_service::content::{ContentBody, DataMessage, GroupContextV2},
//   manager::Registered,
//   proto::Content,
//   store::{Store, Thread},
// };
//
// use presage_store_sqlite::SqliteStoreError;
//
// use sqlx::{query, query_as, query_scalar, types::Json};
//
// use crate::MyManager;
//
// // struct copied from sqlite-store cuz ofc its private
// #[derive(Debug)]
// pub struct SqlMessage {
//   pub ts: u64,
//
//   pub sender_service_id: String,
//   pub sender_device_id: u8,
//   pub destination_service_id: String,
//   pub needs_receipt: bool,
//   pub unidentified_sender: bool,
//
//   pub content_body: Vec<u8>,
//   pub was_plaintext: bool,
// }
//
// pub async fn better_messages(
//   manager: &mut MyManager,
//   thread: &Thread,
//   start: u64,
//   end: u64,
// ) -> Result<Vec<Content>, SqliteStoreError> {
//   let store = manager.store().aci_protocol_store().store;
//
//   let (group_master_key, recipient_id) = thread.unzip();
//
//   let (start_incl, start_excl) = range.start_bound().into_sql_bound();
//   let (end_incl, end_excl) = range.end_bound().into_sql_bound();
//
//   let rows = query_as!(
//     SqlMessage,
//     r#"SELECT
//                 ts AS "ts: _",
//                 sender_service_id,
//                 sender_device_id AS "sender_device_id: _",
//                 destination_service_id,
//                 needs_receipt,
//                 unidentified_sender,
//                 content_body,
//                 was_plaintext
//             FROM thread_messages
//             WHERE thread_id = (
//                 SELECT id FROM threads WHERE group_master_key = ? OR recipient_id = ?)
//                 AND coalesce(ts > ?, ts >= ?, true)
//                 AND coalesce(ts < ?, ts <= ?, true)
//             ORDER BY ts DESC"#,
//     group_master_key,
//     recipient_id,
//     start_incl,
//     start_excl,
//     end_incl,
//     end_excl
//   )
//   .fetch_all(&store.db())
//   .await?;
//
//   Ok(Box::new(rows.into_iter().map(TryInto::try_into)))
// }
