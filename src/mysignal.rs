use tokio;
// use tokio::runtime::Builder;
use tokio::sync::mpsc;
// use tokio::task::LocalSet;
use tokio::task::spawn_local;

use crate::logger::Logger;
use crate::signal::Cmd;
use crate::signal::run;
use crate::update::Action;

use presage::{Manager, manager::Registered};
use presage_store_sqlite::{OnNewIdentity, SqliteStore};

pub struct SignalSpawner {
  send: mpsc::UnboundedSender<Cmd>,
}

impl SignalSpawner {
  // pub fn new(output: mpsc::UnboundedSender<Action>) -> Self {
  //   let (send, mut recv) = mpsc::unbounded_channel::<Cmd>();
  //
  //   let rt = Builder::new_current_thread().enable_all().build().unwrap();
  //
  //   std::thread::spawn(move || {
  //     let local = LocalSet::new();
  //
  //     local.spawn_local(async move {
  //       let db_path = "/home/mqngo/Coding/rust/signal-tui/plzwork.db3";
  //
  //       // UNWRAPPING ERROR NOT PRODUCTION READY!!!
  //       let config_store = SqliteStore::open_with_passphrase(&db_path, "secret".into(), OnNewIdentity::Trust)
  //         .await
  //         .unwrap();
  //
  //       while let Some(new_task) = recv.recv().await {
  //         Logger::log(format!("we gyatt a message but before"));
  //         let cloned_output = output.clone();
  //         let cloned_store = config_store.clone();
  //         _ = run(new_task, cloned_store, cloned_output).await;
  //       }
  //       // If the while loop returns, then all the LocalSpawner
  //       // objects have been dropped.
  //     });
  //
  //     // This will return once all senders are dropped and all
  //     // spawned tasks have returned.
  //     Logger::log(format!("blocking on thread ---"));
  //     rt.block_on(local);
  //   });
  //
  //   Self { send }
  // }

  pub fn new(output: mpsc::UnboundedSender<Action>) -> Self {
    let (send, mut recv) = mpsc::unbounded_channel::<Cmd>();

    spawn_local(async move {
      let db_path = "/home/mqngo/Coding/rust/signal-tui/plzwork.db3";

      let config_store = SqliteStore::open_with_passphrase(&db_path, "secret".into(), OnNewIdentity::Trust)
        .await
        .unwrap();

      let mut manager = Manager::load_registered(config_store).await.expect("cant be fucked");

      while let Some(new_task) = recv.recv().await {
        // Logger::log(format!("we gyatt a message but before"));
        let cloned_output = output.clone();

        _ = run(&mut manager, new_task, cloned_output).await;
      }
      // If the while loop returns, then all the LocalSpawner
      // objects have been dropped.
    });

    // This will return once all senders are dropped and all
    // spawned tasks have returned.

    Self { send }
  }

  // #[tokio::main]
  // pub async fn start(input: output: mpsc::UnboundedSender<Action>) {
  //   let db_path = "/home/mqngo/Coding/rust/signal-tui/plzwork.db3";
  //
  //   // UNWRAPPING ERROR NOT PRODUCTION READY!!!
  //   let config_store = SqliteStore::open_with_passphrase(&db_path, "secret".into(), OnNewIdentity::Trust)
  //     .await
  //     .unwrap();
  //
  //   while let Some(new_task) = recv.recv().await {
  //     Logger::log(format!("we gyatt a message but before"));
  //     let cloned_output = output.clone();
  //     let cloned_store = config_store.clone();
  //     _ = run(new_task, cloned_store, cloned_output).await;
  //   }
  //   // If the while loop returns, then all the LocalSpawner
  //   // objects have been dropped.
  // }

  pub fn spawn(&self, task: Cmd) {
    self.send.send(task).expect("Thread with LocalSet has shut down.");
  }
}

// fn try_from(content: &Content) -> Result<Thread, UuidError> {
//   match &content.body {
//
//     // [1-1] Message sent by us with another device
//     ContentBody::SynchronizeMessage(SyncMessage {
//       sent:
//         Some(Sent {
//                   destination_service_id: Some(uuid),
//                   ..
//               }),
//           ..
//       }) => Ok(Self::Contact(Uuid::parse_str(uuid)?)),
//
//       // [Group] message from somebody else
//       ContentBody::DataMessage(DataMessage {
//           group_v2:
//               Some(GroupContextV2 {
//                   master_key: Some(key),
//                   ..
//               }),
//           ..
//     })
//
//       // [Group] message sent by us with another device
//       | ContentBody::SynchronizeMessage(SyncMessage {
//           sent:
//               Some(Sent {
//                   message:
//                       Some(DataMessage {
//                           group_v2:
//                               Some(GroupContextV2 {
//                                   master_key: Some(key),
//                                   ..
//                               }),
//                           ..
//                       }),
//                   ..
//               }),
//           ..
//       })
//       // [Group] message edit sent by us with another device
//       | ContentBody::SynchronizeMessage(SyncMessage {
//           sent:
//               Some(Sent {
//                   edit_message:
//                       Some(EditMessage {
//                           data_message:
//                               Some(DataMessage {
//                                   group_v2:
//                                       Some(GroupContextV2 {
//                                           master_key: Some(key),
//                                           ..
//                                       }),
//                                   ..
//                               }),
//                           ..
//                       }),
//                   ..
//               }),
//           ..
//       })
//       // [Group] Message edit sent by somebody else
//       | ContentBody::EditMessage(EditMessage {
//           data_message:
//               Some(DataMessage {
//                   group_v2:
//                       Some(GroupContextV2 {
//                           master_key: Some(key),
//                           ..
//                       }),
//                   ..
//               }),
//           ..
//       }) => Ok(Self::Group(
//           key.clone()
//               .try_into()
//               .expect("Group master key to have 32 bytes"),
//       )),
//       // [1-1] Any other message directly to us
//       _ => {
//         let sender = content.metadata.sender.raw_uuid();
//         let destination = content.metadata.destination.raw_uuid();
//
//         if sender != destination {
//
//         }
//
//         Ok(Thread::Contact())},
//         }
// }
