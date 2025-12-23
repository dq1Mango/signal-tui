use crossterm::execute;
use presage::proto::data_message::Quote;
use presage::proto::sync_message::Sent;
use tokio::sync::mpsc::UnboundedSender;

use chrono::TimeDelta;

use crossterm::clipboard::CopyToClipboard;
use crossterm::event::{self, Event, EventStream, KeyCode};

use futures::{StreamExt, future::FutureExt};

// use presage::model::messages::Received;
use presage::libsignal_service::content::{Content, ContentBody};
use presage::libsignal_service::prelude::ProfileKey;
use presage::proto::receipt_message::Type;
use presage::proto::{DataMessage, ReceiptMessage, SyncMessage};
use presage::store::ContentExt;
use presage::store::Thread;

use std::sync::Arc;

use crate::logger::Logger;
use crate::*;

#[derive(PartialEq, Debug)]
pub enum LinkingAction {
  Url(Url),
  Success,
  Fail,
}

#[derive(Debug)]
pub enum Action {
  Type(char),
  Backspace,
  Send,
  Nvm,

  Scroll(isize),
  ScrollGroup(isize),
  ScrollOptions(isize),

  PickOption,
  DoOption(MessageOption),

  SetMode(Mode),
  SetFocus(Focus),

  // Message(Content),
  Receive(Received),
  ReceiveBatch(Vec<Content>),

  Link(LinkingAction),
  Quit,
}

#[derive(Debug, Copy, Clone)]
pub enum MessageOption {
  Reply,
  Edit,
  Copy,
  Info,
  Delete,
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
      KeyCode::Char('o') => Some(Action::SetMode(Mode::MessageOptions)),

      KeyCode::Char('S') => Some(Action::SetFocus(Focus::Settings)),

      KeyCode::Char('x') => Some(Action::Nvm),

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

    Mode::MessageOptions => match key.code {
      KeyCode::Char('q') => Some(Action::Quit),
      KeyCode::Esc => Some(Action::SetMode(Mode::Normal)),
      KeyCode::Char('j') => Some(Action::ScrollOptions(1)),
      KeyCode::Char('k') => Some(Action::ScrollOptions(-1)),
      KeyCode::Enter => Some(Action::PickOption),
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
        chat.load_more_messages(spawner, TimeDelta::try_hours(12).unwrap());
      }
      //model.current_chat().location.requested_scroll = lines,
    }

    Action::ScrollGroup(direction) => {
      model.chat_index = (model.chat_index as isize + direction).rem_euclid(model.chats.len() as isize) as usize;
      //.clamp(0, model.chats.len() as isize - 1) as usize
    }

    // this one pissed me off a little cuz u should be able to do it all in one line
    Action::ScrollOptions(scroll) => {
      // let index = model.current_chat().location.index;
      let length = match model.current_chat().selected_message()?.metadata {
        Metadata::MyMessage(_) => 5,
        Metadata::NotMyMessage(_) => 3,
      };

      let options = &mut model.current_chat().message_options;
      options.index = (options.index as isize + scroll).rem_euclid(length) as usize;
    }

    Action::SetMode(new_mode) => {
      *model.mode.lock().unwrap() = new_mode.clone();
      model.pinned_mode = new_mode;

      // alright this is just getting the littlest bit of ugly now
      if model.pinned_mode == Mode::MessageOptions {
        let selected_message = model.current_chat().selected_message()?;
        let mine = selected_message.is_mine();
        let ts = selected_message.ts();
        model.current_chat().message_options.open(ts, mine);
      } else {
        model.current_chat().message_options.close();
      }
    }

    Action::Nvm => {
      let input = &mut model.current_chat().text_input;
      match input.mode {
        TextInputMode::Replying => {
          input.mode = TextInputMode::Normal;
        }
        TextInputMode::Editing => {
          input.mode = TextInputMode::Normal;
          input.clear();
        }
        TextInputMode::Normal => {}
      }
    }

    // Action::SetFocus(new_focus) => model.focus = new_focus,
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

    Action::ReceiveBatch(received) => {
      for message in received {
        handle_message(model, message);
      }
    }

    Action::PickOption => return Some(model.current_chat().message_options.select()),
    Action::DoOption(option) => return handle_option(model, spawner, option),

    Action::Quit => {
      // You can handle cleanup and exit here
      // -- im ok thanks tho
      model.running_state = RunningState::OhShit;
    }

    _ => {}
  }

  None
}

pub fn handle_option(model: &mut Model, spawner: &SignalSpawner, option: MessageOption) -> Option<Action> {
  let chat = model.current_chat();
  let message = chat.find_message(chat.message_options.timestamp)?;

  // ensure the optino we receive is actually valid for the message
  // ie. cant edit / delete someone elses message
  if let Metadata::NotMyMessage(_) = message.metadata {
    match &option {
      &MessageOption::Edit => {
        Logger::log(format!("invalid message option: {:?}", option));
        return None;
      }
      &MessageOption::Delete => {
        Logger::log(format!("invalid message option: {:?}", option));
        return None;
      }
      _ => {}
    }
  }

  chat.message_options.close();

  match option {
    MessageOption::Copy => {
      let result = execute!(
        std::io::stdout(),
        CopyToClipboard::to_clipboard_from(&model.current_chat().selected_message().expect("kaboom").body.body)
      );

      if let Err(error) = result {
        Logger::log(error)
      }

      Some(Action::SetMode(Mode::Normal))
    }
    MessageOption::Reply => {
      chat.text_input.mode = TextInputMode::Replying;
      Some(Action::SetMode(Mode::Insert))
    }
    MessageOption::Edit => {
      // kinda gotta find the message twice sometimes cuz "cant have more than one mutable borrow
      // yaaaaaaaaaayy..."
      let body = chat.find_message(chat.message_options.timestamp)?.body.body.clone();
      chat.text_input.set_content(body);

      chat.text_input.mode = TextInputMode::Editing;
      Some(Action::SetMode(Mode::Insert))
    }
    MessageOption::Delete => {
      let ts = model.current_chat().message_options.timestamp;
      spawner.spawn(Cmd::DeleteMessage {
        thread: model.current_chat().thread.clone(),
        target_timestamp: ts,
      });

      model.current_chat().delete_message(ts);

      Some(Action::SetMode(Mode::Normal))
    }
    _ => None,
  }
}

fn handle_message(model: &mut Model, content: Content) -> Option<Action> {
  // Logger::log(format!("DataMessage: {:#?}", content.clone()));

  let ts = content.timestamp();
  let timestamp = DateTime::from_timestamp_millis(ts as i64).expect("this happens too often");

  let Ok(mut thread) = Thread::try_from(&content) else {
    Logger::log("failed to derive thread from content".to_string());
    return None;
  };

  match content.body {
    ContentBody::DataMessage(DataMessage {
      body: Some(body),
      quote,
      ..
    }) => {
      // Logger::log(format!("DataMessage: {:#?}", body.clone()));
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

      let quote = if let Some(Quote { id, .. }) = quote { id } else { None };

      let message = Message {
        body: MultiLineString::new(&body),
        metadata,
        quote,
      };

      let Some(chat) = model.find_chat(&thread) else {
        Logger::log(format!("Could not find a chat that matched the id: {:#?}", thread));
        return None;
      };

      chat.insert_message(message);

      // insert_message(model, data, thread, ts, mine)
    }

    // maybe this is a ratchet acting as a delivery receipt? maybe...?
    ContentBody::DataMessage(DataMessage {
      body: None,
      flags: Some(4),
      ..
    }) => {
      Logger::log("found fake receipt".to_string());
      // some flex-tape on the thread derivation
      // let mut mine = false;
      if let Thread::Contact(uuid) = thread {
        if uuid == model.account.uuid {
          thread = Thread::Contact(content.metadata.destination.raw_uuid());
          // mine = true;
        }
      }

      let Some(chat) = model.find_chat(&thread) else {
        Logger::log(format!("Could not find a chat that matched the id: {:#?}", thread));
        return None;
      };

      let message = chat.last_message_mut();
      if let Metadata::MyMessage(metadata) = &mut message?.metadata {
        if metadata.delivered_to.len() == 0 {
          metadata.delivered_to.push(Receipt {
            timestamp: timestamp,
            sender: content.metadata.sender.raw_uuid(),
          });
        }
      }

      // insert_message(model, data, thread, ts, mine)
    }
    ContentBody::SynchronizeMessage(data) => {
      match data {
        SyncMessage {
          sent:
            Some(Sent {
              message:
                Some(DataMessage {
                  body: Some(body),
                  quote,
                  ..
                }),
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
            Logger::log(format!("Could not find a chat that matched the id: {:#?}", thread));
            return None;
          };

          // for uuid in chat.participants.members {
          //   if !metadata.read_by.contains(&(uuid, _)) {
          //     metadata.read_by.push((uuid, None));
          //     metadata.delivered_to.push((uuid, None));
          //   }
          // }

          let quote = if let Some(Quote { id, .. }) = quote { id } else { None };

          let message = Message {
            body: MultiLineString::new(&body),
            metadata,
            quote,
          };

          chat.insert_message(message);
        }
        _ => {}
      }
    }
    ContentBody::ReceiptMessage(receipt_message) => {
      if let Some(chat) = model.find_chat(&thread) {
        let kind = receipt_message.r#type();
        let times = receipt_message.timestamp;

        for time in times {
          if let Some(message) = chat.find_message(time) {
            if let Metadata::MyMessage(MyMessage {
              read_by, delivered_to, ..
            }) = &mut message.metadata
            {
              let receipt = Receipt {
                sender: content.metadata.sender.raw_uuid(),
                timestamp: timestamp,
              };

              match kind {
                Type::Delivery => delivered_to.push(receipt),
                Type::Read => read_by.push(receipt),
                _ => {}
              }
            }
          } else {
            // Logger::log(format!(
            //   "didnt find message in thread: {:#?} with timestamp: {:?}",
            //   thread, ts
            // ));
          }
        }
      } else {
        Logger::log(format!("didnt find chat with thread: {:#?}", thread));
      }
    }
    _ => {}
  }

  None
}

pub async fn update_contacts(model: &mut Model, spawner: &SignalSpawner) -> anyhow::Result<()> {
  Logger::log("updating contacts".to_string());
  for contact in spawner.list_contacts().await? {
    // Logger::log(format!("{}", contact.inbox_position));
    if model.contacts.contains_key(&contact.uuid) {
      Logger::log("already_gyatt key".to_string());
      continue;
    } else {
      let profile_key = match contact.profile_key.clone().try_into() {
        Ok(bytes) => Some(ProfileKey::create(bytes)),
        Err(_) => {
          // Logger::log(format!("died on this dude: {:#?}", contact));
          None
        }
      };

      let profile = match spawner.retrieve_profile(contact.uuid, profile_key).await {
        Ok(x) => x,
        Err(_) => continue,
      };

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

impl Model {
  pub async fn update_groups(self: &mut Self, spawner: &SignalSpawner) -> anyhow::Result<()> {
    Logger::log("updating groups".to_string());
    for (key, group) in spawner.list_groups().await {
      if !self.groups.contains_key(&key) {
        self.new_group_chat(key, &group);
      }
      let Some(groups) = Arc::get_mut(&mut self.groups) else {
        Logger::log("didnt get off so easy".to_string());
        continue;
      };

      groups.insert(key, group);
    }
    Ok(())
  }
}
