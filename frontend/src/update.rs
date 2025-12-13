use presage::libsignal_service::protocol::ServiceId;
use presage::proto::sync_message::{Read, Sent};
use tokio::sync::mpsc::UnboundedSender;

use chrono::TimeDelta;
use crossterm::event::{self, Event, EventStream, KeyCode};

use futures::{StreamExt, future::FutureExt};

// use presage::model::messages::Received;
use presage::libsignal_service::content::{Content, ContentBody};
use presage::libsignal_service::prelude::ProfileKey;
use presage::proto::{DataMessage, ReceiptMessage, SyncMessage};
use presage::store::ContentExt;
use presage::store::Thread;

use core::time;
use std::sync::Arc;

use crate::logger::Logger;
use crate::*;

#[derive(PartialEq)]
pub enum LinkingAction {
  Url(Url),
  Success,
  Fail,
}

pub enum Action {
  Type(char),
  Backspace,
  Send,

  Scroll(isize),
  ScrollGroup(isize),

  SetMode(Mode),
  SetFocus(Focus),

  Link(LinkingAction),
  // Message(Content),
  Receive(Received),

  Quit,
}
/// Convert Event to Action
///
/// We don't need to pass in a `model` to this function in this example
/// but you might need it as your project evolves
///
/// (the project evolved (pokemon core))
pub async fn handle_crossterm_events(tx: UnboundedSender<Action>, mode: &Arc<Mutex<Mode>>) {
  let mut reader = EventStream::new();

  loop {
    let event = reader.next().fuse().await;
    match event {
      Some(Ok(event)) => match event {
        Event::Key(key) => {
          if key.kind == event::KeyEventKind::Press {
            if let Some(action) = handle_key(key, mode) {
              let _err = tx.send(action);
            }
          }
        }
        _ => {}
      },
      Some(Err(err)) => Logger::log(format!("Error reading event: {err} ")),
      None => Logger::log(format!("I dont think this should ever happend")),
    }
  }
}

pub fn handle_key(key: event::KeyEvent, mode: &Arc<Mutex<Mode>>) -> Option<Action> {
  // the settings focus isnt super real yet so idk what im doing
  match *mode.lock().unwrap() {
    Mode::Insert => match key.code {
      KeyCode::Esc => Some(Action::SetMode(Mode::Normal)),
      KeyCode::Enter => Some(Action::Send),
      KeyCode::Char(char) => Some(Action::Type(char)),
      // this will not get confusing trust
      KeyCode::Backspace => Some(Action::Backspace),
      _ => None,
    },

    Mode::Normal => match key.code {
      KeyCode::Char('j') => Some(Action::Scroll(-1)),
      KeyCode::Char('k') => Some(Action::Scroll(1)),
      KeyCode::Char('d') => Some(Action::Scroll(-10)),
      KeyCode::Char('u') => Some(Action::Scroll(10)),

      KeyCode::Char('i') => Some(Action::SetMode(Mode::Insert)),
      KeyCode::Char('h') => Some(Action::SetMode(Mode::Groups)),

      KeyCode::Char('S') => Some(Action::SetFocus(Focus::Settings)),

      KeyCode::Char('q') => Some(Action::Quit),
      _ => None,
    },

    Mode::Groups => match key.code {
      KeyCode::Char('j') => Some(Action::ScrollGroup(1)),
      KeyCode::Char('k') => Some(Action::ScrollGroup(-1)),

      KeyCode::Char('l') => Some(Action::SetMode(Mode::Normal)),

      KeyCode::Char('q') => Some(Action::Quit),
      _ => None,
    },

    Mode::Settings => match key.code {
      KeyCode::Char('l') => Some(Action::SetMode(Mode::Normal)),

      KeyCode::Char('q') => Some(Action::Quit),
      _ => None,
    },
  }
}

pub async fn update(model: &mut Model, msg: Action, spawner: &SignalSpawner) -> Option<Action> {
  match msg {
    Action::Type(char) => {
      model.current_chat().text_input.insert_char(char);
    }

    Action::Backspace => model.current_chat().text_input.delete_char(),

    Action::Send => model.current_chat().send(spawner),

    Action::Scroll(lines) => {
      let chat = model.current_chat();
      if chat.messages.len() > 0 {
        chat.location.index =
          (chat.location.index as isize + lines).clamp(0, chat.messages.len() as isize - 1) as usize;
      }

      if chat.location.index == 0 {
        Logger::log("loading messages".to_string());
        chat.load_more_messages(spawner, TimeDelta::try_hours(24).unwrap());
      }
      //model.current_chat().location.requested_scroll = lines,
    }

    Action::ScrollGroup(direction) => {
      model.chat_index = (model.chat_index as isize + direction).rem_euclid(model.chats.len() as isize) as usize;
      //.clamp(0, model.chats.len() as isize - 1) as usize
    }

    Action::SetMode(new_mode) => {
      *model.mode.lock().unwrap() = new_mode.clone();
      model.pinned_mode = new_mode;
    }

    // Action::SetFocus(new_focus) => model.focus = new_focus,
    Action::Quit => {
      // You can handle cleanup and exit here
      // -- im ok thanks tho
      model.running_state = RunningState::OhShit;
    }

    Action::Receive(received) => match received {
      Received::Content(content) => {
        return handle_message(model, *content);
      }
      Received::Contacts => {
        // update our in memory cache of contacts
        _ = update_contacts(model, spawner).await;
      }
      Received::QueueEmpty => {}
    },

    _ => {}
  }

  None
}

// use
//
// fn slices_equal<T>(slice1: Vec<T>, slice2: Vec<T>) -> bool {
//   if slice1.len() != slice1.len() {
//     return false;
//   }
//
//   slice1.iter().cmp()
// }

// pub fn insert_message(model: &mut Model, message: DataMessage, thread: Thread, timestamp: u64, mine: bool) {
//   match thread {
//     Thread::Contact(uuid) => {
//       // Logger::log(format!(
//       //   "thread: {}, with body: {}",
//       //   uuid,
//       //   message.body.clone().unwrap_or("useless message".to_string())
//       // ));
//       for chat in &mut model.chats {
//         // maybe this rust thing isnt so bad (jk lol)
//         if chat.participants.members == [uuid] {
//           chat.insert_message(message, uuid, timestamp, mine);
//           return;
//         }
//       }
//
//       Logger::log(format!("Could not find a chat that matched the id: {}", uuid));
//     }
//     _ => {}
//   }
// }

fn handle_message(model: &mut Model, content: Content) -> Option<Action> {
  Logger::log(format!("{:#?}", content.clone()));
  //
  let ts = content.timestamp();
  let timestamp = DateTime::from_timestamp_millis(ts as i64).expect("this happens too often");

  let Ok(mut thread) = Thread::try_from(&content) else {
    Logger::log("failed to derive thread from content".to_string());
    return None;
  };

  match content.body {
    ContentBody::DataMessage(DataMessage { body: Some(body), .. }) => {
      // some flex-tape on the thread derivation
      let mut mine = false;
      if let Thread::Contact(uuid) = thread {
        if uuid == model.account.uuid {
          thread = Thread::Contact(content.metadata.destination.raw_uuid());
          mine = true;
        }
      }

      let metadata = if mine {
        Metadata::MyMessage(MyMessage {
          sent: timestamp,
          delivered_to: vec![],
          read_by: vec![],
        })
      } else {
        Metadata::NotMyMessage(NotMyMessage {
          sent: timestamp,
          sender: content.metadata.sender.raw_uuid(),
        })
      };

      let Some(chat) = model.find_chat(&thread) else {
        Logger::log(format!(
          "Could not find a chat that matched the id: {:#?}",
          thread
        ));
        return None;
      };

      chat.insert_message(&body, metadata);

      // insert_message(model, data, thread, ts, mine)
    }
    ContentBody::SynchronizeMessage(data) => {
      match data {
        SyncMessage {
          sent:
            Some(Sent {
              message: Some(DataMessage { body: Some(body), .. }),
              ..
            }),
          // read: read,
          ..
        } => {
          let read_by = Vec::new();
          // for receipt in read {
          //   let Some(aci) = receipt.sender_aci else {
          //     continue;
          //   };
          //   let Some(timestamp) = receipt.timestamp else { continue };
          //   let Some(aci) = ServiceId::parse_from_service_id_string(&aci) else {
          //     Logger::log("plz no".to_string());
          //     return None;
          //   };
          //   read_by.push(Receipt {
          //     sender: aci.raw_uuid(),
          //     timestamp: DateTime::from_timestamp_millis(timestamp as i64).expect("i think i gotta ditch chrono"),
          //   });
          // }
          let metadata = Metadata::MyMessage(MyMessage {
            sent: timestamp,
            delivered_to: read_by.clone(),
            read_by: read_by,
          });

          let Some(chat) = model.find_chat(&thread) else {
            Logger::log(format!(
              "Could not find a chat that matched the id: {:#?}",
              thread
            ));
            return None;
          };

          // for uuid in chat.participants.members {
          //   if !metadata.read_by.contains(&(uuid, _)) {
          //     metadata.read_by.push((uuid, None));
          //     metadata.delivered_to.push((uuid, None));
          //   }
          // }
          chat.insert_message(&body, metadata);
        }
        // SyncMessage {
        //   sent: None,
        //   read: receipts,
        //   ..
        // } => {
        //   if let Some(chat) = model.find_chat(thread) {
        //     for receipt in receipts {
        //       chat.add_receipt(receipt.timestamp);
        //     }
        //   }
        // }
        _ => {}
      }
      // if let Some(sent) = data.sent {
      //   if let Some(message) = sent.message {
      //   }
      // }
    }
    ContentBody::ReceiptMessage(ReceiptMessage {
      r#type: Some(_raw_type),
      timestamp: times,
    }) => {
      if let Some(chat) = model.find_chat(&thread) {
        for time in times {
          if let Some(message) = chat.find_message(time) {
            if let Metadata::MyMessage(MyMessage { read_by, .. }) = &mut message.metadata {
              read_by.push(Receipt {
                sender: content.metadata.sender.raw_uuid(),
                timestamp: DateTime::from_timestamp_millis(ts as i64).expect("yeah this is getting old"),
              });
            }
          } else {
            Logger::log("didnt find chat".to_string());
          }
        }
      }
    }
    _ => {}
  }

  None
}

pub async fn update_contacts(model: &mut Model, spawner: &SignalSpawner) -> anyhow::Result<()> {
  Logger::log("i gyatt called".to_string());
  for contact in spawner.list_contacts().await? {
    if model.contacts.contains_key(&contact.uuid) {
      continue;
    } else {
      let profile_key = Some(ProfileKey::create(
        contact.profile_key.try_into().expect("we tried"),
      ));
      let profile = spawner.retrieve_profile(contact.uuid, profile_key).await?;

      let Some(contacts) = Arc::get_mut(&mut model.contacts) else {
        Logger::log("didnt get off so easy".to_string());
        return Ok(());
      };

      contacts.insert(contact.uuid, profile.clone());

      model.new_dm_chat(profile, contact.uuid);
    }
  }
  Ok(())
}

// async fn print_message(manager: &Manager<S, Registered>, notifications: bool, content: &Content) {
//   let Ok(thread) = Thread::try_from(content) else {
//     warn!("failed to derive thread from content");
//     return;
//   };
//
//   async fn format_data_message(
//     thread: &Thread,
//     data_message: &DataMessage,
//     manager: &Manager<S, Registered>,
//   ) -> Option<String> {
//     match data_message {
//       DataMessage {
//         quote: Some(Quote {
//           text: Some(quoted_text), ..
//         }),
//         body: Some(body),
//         ..
//       } => Some(format!("Answer to message \"{quoted_text}\": {body}")),
//       DataMessage {
//         reaction: Some(Reaction {
//           target_sent_timestamp: Some(ts),
//           emoji: Some(emoji),
//           ..
//         }),
//         ..
//       } => {
//         let Ok(Some(message)) = manager.store().message(thread, *ts).await else {
//           warn!(%thread, sent_at = ts, "no message found in thread");
//           return None;
//         };
//
//         let ContentBody::DataMessage(DataMessage { body: Some(body), .. }) = message.body else {
//           warn!("message reacted to has no body");
//           return None;
//         };
//
//         Some(format!("Reacted with {emoji} to message: \"{body}\""))
//       }
//       DataMessage { body: Some(body), .. } => Some(body.to_string()),
//       _ => Some("Empty data message".to_string()),
//     }
//   }
//
//   async fn format_contact(uuid: &Uuid, manager: &Manager<S, Registered>) -> String {
//     manager
//       .store()
//       .contact_by_id(uuid)
//       .await
//       .ok()
//       .flatten()
//       .filter(|c| !c.name.is_empty())
//       .map(|c| format!("{}: {}", c.name, uuid))
//       .unwrap_or_else(|| uuid.to_string())
//   }
//
//   async fn format_group(key: [u8; 32], manager: &Manager<S, Registered>) -> String {
//     manager
//       .store()
//       .group(key)
//       .await
//       .ok()
//       .flatten()
//       .map(|g| g.title)
//       .unwrap_or_else(|| "<missing group>".to_string())
//   }
//
//   enum Msg<'a> {
//     Received(&'a Thread, String),
//     Sent(&'a Thread, String),
//   }
//
//   if let Some(msg) = match &content.body {
//     ContentBody::NullMessage(_) => Some(Msg::Received(&thread, "Null message (for example deleted)".to_string())),
//     ContentBody::DataMessage(data_message) => format_data_message(&thread, data_message, manager)
//       .await
//       .map(|body| Msg::Received(&thread, body)),
//     ContentBody::EditMessage(EditMessage {
//       data_message: Some(data_message),
//       ..
//     }) => format_data_message(&thread, data_message, manager)
//       .await
//       .map(|body| Msg::Received(&thread, body)),
//     ContentBody::EditMessage(EditMessage { .. }) => None,
//     ContentBody::SynchronizeMessage(SyncMessage {
//       sent: Some(Sent {
//         message: Some(data_message),
//         ..
//       }),
//       ..
//     }) => format_data_message(&thread, data_message, manager)
//       .await
//       .map(|body| Msg::Sent(&thread, body)),
//     ContentBody::SynchronizeMessage(SyncMessage {
//       sent:
//         Some(Sent {
//           edit_message: Some(EditMessage {
//             data_message: Some(data_message),
//             ..
//           }),
//           ..
//         }),
//       ..
//     }) => format_data_message(&thread, data_message, manager)
//       .await
//       .map(|body| Msg::Sent(&thread, body)),
//     ContentBody::SynchronizeMessage(SyncMessage { .. }) => None,
//     ContentBody::CallMessage(_) => Some(Msg::Received(&thread, "is calling!".into())),
//     ContentBody::TypingMessage(_) => Some(Msg::Received(&thread, "is typing...".into())),
//     ContentBody::ReceiptMessage(ReceiptMessage {
//       r#type: receipt_type,
//       timestamp,
//     }) => Some(Msg::Received(
//       &thread,
//       format!(
//         "got {:?} receipt for messages sent at {timestamp:?}",
//         receipt_message::Type::try_from(receipt_type.unwrap_or_default()).unwrap()
//       ),
//     )),
//     ContentBody::StoryMessage(story) => Some(Msg::Received(&thread, format!("new story: {story:?}"))),
//     ContentBody::PniSignatureMessage(_) => Some(Msg::Received(&thread, "got PNI signature message".into())),
//   } {
//     let ts = content.timestamp();
//     let (prefix, body) = match msg {
//       Msg::Received(Thread::Contact(sender), body) => {
//         let contact = format_contact(sender, manager).await;
//         (format!("From {contact} @ {ts}: "), body)
//       }
//       Msg::Sent(Thread::Contact(recipient), body) => {
//         let contact = format_contact(recipient, manager).await;
//         (format!("To {contact} @ {ts}"), body)
//       }
//       Msg::Received(Thread::Group(key), body) => {
//         let sender = format_contact(&content.metadata.sender.raw_uuid(), manager).await;
//         let group = format_group(*key, manager).await;
//         (format!("From {sender} to group {group} @ {ts}: "), body)
//       }
//       Msg::Sent(Thread::Group(key), body) => {
//         let group = format_group(*key, manager).await;
//         (format!("To group {group} @ {ts}"), body)
//       }
//     };
//
//     println!("{prefix} / {body}");
//
//     if notifications {
//       if let Err(error) = Notification::new().summary(&prefix).body(&body).icon("presage").show() {
//         error!(%error, "failed to display desktop notification");
//       }
//     }
//   }
// }
