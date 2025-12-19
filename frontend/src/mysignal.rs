use presage::libsignal_service::zkgroup::GroupMasterKeyBytes;
use presage::model::groups::Group;
use presage_store_sqlite::SqliteStoreError;
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
use crate::signal::list_groups;
use crate::signal::process_incoming_message;
use crate::signal::retrieve_profile;
use crate::signal::run;
use crate::update::Action;

use futures::StreamExt;
use futures::pin_mut;
use tokio::select;

use crate::MyManager;
use presage::Error;
use presage::model::contacts::Contact;
// use presage::store::ContentsStore;
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

pub struct SignalSpawner {
  send: mpsc::UnboundedSender<Cmd>,
  contact_requests: Requester<Result<Vec<Contact>, Error<SqliteStoreError>>>,
  group_requests: Requester<Vec<(GroupMasterKeyBytes, Group)>>,
  profile_requests: mpsc::UnboundedSender<ProfileRequest>,
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

  pub fn new(mut manager: MyManager, output: mpsc::UnboundedSender<Action>) -> Self {
    let (send, mut recv) = mpsc::unbounded_channel::<Cmd>();

    // i feel like the compiler should be able to figure out these types
    let (contacts_sender, mut contact_requests) =
      mpsc::unbounded_channel::<oneshot::Sender<Result<Vec<Contact>, Error<SqliteStoreError>>>>();
    let (groups_sender, mut group_requests) =
      mpsc::unbounded_channel::<oneshot::Sender<Vec<(GroupMasterKeyBytes, Group)>>>();

    let (profile_sender, mut profile_requests) = mpsc::unbounded_channel();

    // let (message_tx, mut message_rx) = mpsc::unbounded_channel();

    spawn_local(async move {
      // initialize message stream
      let messages = manager
        .receive_messages()
        .await
        .expect("failed to initialize messages stream");
      pin_mut!(messages);

      // handle messages in a different "thread" to convert stream to channel
      // spawn_local(async move {
      //
      //   while let Some(message) = messages.next().await {
      //     _ = message_tx.send(message);
      //   }
      // });

      // let max_messages_in_a_row = 1;
      // let max_commands_in_a_row = 3;
      let attachments_tmp_dir = attachments_tmp_dir().expect("this is dumb");

      // let mut counter;

      // should enable some gracefull shutdown
      while !output.is_closed() && !recv.is_closed() {
        // which we can only wait and see if this was a bad choice

        select! {
          Some(contacts_output) = contact_requests.recv() => {
          let contacts = get_contacts(&manager).await;

          _ = contacts_output.send(contacts);
          }

          Some(groups_output) = group_requests.recv() => {
            _ = groups_output.send(list_groups(&manager).await);
          }

          // profile requestss
          Some(profile_request) = profile_requests.recv() =>
          {
            let ProfileRequest {
            output,
            uuid,
            profile_key,
          } = profile_request;
            _ = output.send(retrieve_profile(&mut manager, uuid, profile_key).await);
          }

          Some(content) = messages.next() => {
            Logger::log("mhhhh some juicy content for you");
            match &content {
              Received::QueueEmpty => {
                _ = output.send(Action::Receive(Received::QueueEmpty));
                // break;
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

          }

          Some(task) = recv.recv() => {
            _ = run(&mut manager, task, output.clone()).await;
            // if counter > max_commands_in_a_row {
            //   break;
            // }
          }
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
      group_requests: groups_sender,
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

  pub async fn list_contacts(&self) -> Result<Vec<Contact>, Error<SqliteStoreError>> {
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

    Logger::log("awaiting...");
    return rx.await.expect("kaboom");
  }

  pub async fn list_groups(&self) -> Vec<(GroupMasterKeyBytes, Group)> {
    let (tx, rx) = oneshot::channel();

    _ = self.group_requests.send(tx);

    return rx.await.expect("kaboom once again");
  }

  pub fn sync_contacts(&self) {
    _ = self.send.send(Cmd::SyncContacts);
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
