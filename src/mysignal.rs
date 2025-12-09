use tokio;
// use tokio::runtime::Builder;
use tokio::sync::mpsc;
// use tokio::task::LocalSet;
use tokio::sync::oneshot;
use tokio::task::spawn_local;

use crate::Profile;
use crate::ProfileKey;
use crate::Received;
use crate::Uuid;
use crate::logger::Logger;
use crate::signal::Cmd;
use crate::signal::attachments_tmp_dir;
use crate::signal::get_contacts;
use crate::signal::process_incoming_message;
use crate::signal::retrieve_profile;
use crate::signal::run;
use crate::update::Action;

use futures::StreamExt;
use futures::pin_mut;

use presage::Error;
use presage::model::contacts::Contact;
use presage::store::Store;
use presage::{Manager, manager::Registered};
use presage_store_sqlite::SqliteStore;

// pub struct Task<Command, Data> {
//   cmd: Cmd,
//   output: oneshot::Sender<Box<T>>,
// }

type Requester<Data> = mpsc::UnboundedSender<oneshot::Sender<Data>>;

// struct ContactRequest {
//   output: oneshot::Sender<>
// }

struct ProfileRequest {
  output: oneshot::Sender<anyhow::Result<Profile>>,
  uuid: Uuid,
  profile_key: Option<ProfileKey>,
}

pub struct SignalSpawner<S: Store> {
  send: mpsc::UnboundedSender<Cmd>,
  contact_requests: Requester<Result<Vec<Contact>, Error<S::Error>>>,
  profile_requests: mpsc::UnboundedSender<ProfileRequest>,
}

impl<S: Store> SignalSpawner<S> {
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

  pub fn new(mut manager: Manager<S, Registered>, output: mpsc::UnboundedSender<Action>) -> Self {
    let (send, mut recv) = mpsc::unbounded_channel::<Cmd>();
    let (contacts_sender, mut contact_requests) =
      mpsc::unbounded_channel::<oneshot::Sender<Result<Vec<Contact>, Error<S::Error>>>>();
    let (profile_sender, mut profile_requests) = mpsc::unbounded_channel();

    spawn_local(async move {
      let max_messages_in_a_row = 67;
      let attachments_tmp_dir = attachments_tmp_dir().expect("this is dumb");

      // should enable some gracefull shutdown
      while !output.is_closed() && !recv.is_closed() {
        // currently requests to the manager are processed in a distinct priority,
        // which we can only wait and see if this was a bad choice
        while let Ok(contacts_output) = contact_requests.try_recv() {
          _ = contacts_output.send(get_contacts(&manager).await);
        }

        while let Ok(ProfileRequest {
          output,
          uuid,
          profile_key,
        }) = profile_requests.try_recv()
        {
          _ = output.send(retrieve_profile(&mut manager, uuid, profile_key).await);
        }

        // probably should not be re-making this stream each iteration but im sure its fine
        let messages = manager
          .receive_messages()
          .await
          .expect("failed to initialize messages stream");

        pin_mut!(messages);

        let mut counter = 0;
        while let Some(content) = messages.next().await {
          match &content {
            Received::QueueEmpty => {
              _ = output.send(Action::Receive(Received::QueueEmpty));
              break;
            }
            Received::Contacts => {
              //println!("got contacts synchronization"),
            }
            Received::Content(content) => {
              // this better be fast lmao
              process_incoming_message(&mut manager, attachments_tmp_dir.path(), false, &content).await
            }
          }

          _ = output.send(Action::Receive(content));

          counter += 1;
          if counter > max_messages_in_a_row {
            break;
          }
        }

        while let Ok(task) = recv.try_recv() {
          Logger::log("gyatt task".to_string());
          _ = run(&mut manager, task, output.clone()).await;
        }
      }
      Logger::log("gracefully shutdown ... (hopefully)".to_string());

      // while let Some(new_task) = recv.recv().await {
      //   // Logger::log(format!("we gyatt a message but before"));
      //   let cloned_output = output.clone();
      //
      //   let error = run(&mut manager, new_task, cloned_output).await;
      //
      //   // if let Cmd::Send { .. } = new_task {
      //   Logger::log(format!("{:?}", error));
      //   // }
      // }
      // If the while loop returns, then all the LocalSpawner
      // objects have been dropped.
    });

    // This will return once all senders are dropped and all
    // spawned tasks have returned.

    Self {
      send: send,
      contact_requests: contacts_sender,
      profile_requests: profile_sender,
    }
  }

  // #[tokio::main]
  // pub async fn start(input: output: mpsc::UnboundedSender<Action>) {
  //   let db_path = "/home/mqngo/Coding/rust/signal-tui/plzwork.db3";
  //
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

  pub async fn list_contacts(&self) -> Result<Vec<Contact>, Error<S::Error>> {
    let (tx, rx) = oneshot::channel();

    _ = self.contact_requests.send(tx);

    return rx.await.expect("kaboom");
  }

  pub async fn retrieve_profile(&self, uuid: Uuid, profile_key: Option<ProfileKey>) -> anyhow::Result<Profile> {
    let (tx, rx) = oneshot::channel();

    _ = self.profile_requests.send(ProfileRequest {
      output: tx,
      uuid,
      profile_key,
    });

    return rx.await.expect("kaboom");
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
