mod logger;
mod model;
mod mysignal;
mod signal;
#[cfg(test)]
mod tests;

mod update;

use std::{
  cmp,
  collections::HashMap,
  fmt::Debug,
  hash::Hash,
  io::{self, Stdout},
  sync::{Arc, Mutex},
  vec,
};

use crossterm::{ExecutableCommand, cursor};
use presage::{
  libsignal_service::{
    Profile,
    configuration::SignalServers,
    prelude::{ProfileKey, Uuid},
    proto,
    zkgroup::GroupMasterKeyBytes,
  },
  model::groups::Group,
  store::Thread,
};

use presage::manager::Manager;
use presage::model::messages::Received;
use presage::store::{StateStore, Store};
use presage_store_sqlite::{OnNewIdentity, SqliteStore};
// use crate::database::{OnNewIdentity, SqliteStore};

use ratatui::{
  Frame,
  buffer::Buffer,
  // crossterm::style::Color,
  layout::{Constraint, Direction, Flex, Layout, Position, Rect},
  style::{Color, Modifier, Style, Stylize},
  symbols::border,
  text::{Line, Span},
  widgets::{Block, Gauge, Paragraph, Widget},
};

use chrono::{DateTime, TimeDelta, Utc};
use tokio::sync::mpsc;
use url::Url;
// use ratatui_image::{StatefulImage, picker::Picker, protocol::StatefulProtocol};

use qrcodegen::QrCode;
use qrcodegen::QrCodeEcc;
// use crate::signal::*;
use crate::signal::{default_db_path, get_quote, link_device};
use crate::update::*;
use crate::{
  logger::Logger, model::MultiLineString, mysignal::SignalSpawner, signal::Cmd,
  update::LinkingAction,
};

// there are three different models to represent all the parts of linking a device, loading
// past messages, and normal operation, which is ugly dont get me wrong, but i feel like
// cramming them all in one struct would be worse

pub type MyManager = Manager<SqliteStore, presage::manager::Registered>;

// #[derive(Debug, Default)]
pub struct Model {
  running_state: RunningState,
  mode: Arc<Mutex<Mode>>,
  pinned_mode: Mode,
  contacts: Contacts,
  groups: Groups,
  // groups: Vec<Group,
  chats: Vec<Chat>,
  chat_index: usize,
  account: Account,
}

pub struct LinkState {
  url: Option<Url>,
}

struct LoadState {
  raw_duration: Option<u64>,
  latest_timestamp: Option<u64>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum RunningState {
  #[default]
  Running,
  OhShit,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub enum Mode {
  #[default]
  Normal,
  Insert,
  Groups,
  Settings,
  MessageOptions,
}

#[derive(Default, Debug, PartialEq)]
pub enum Focus {
  #[default]
  Chats,
  Settings,
  Groups,
}

// #[derive(Debug, Default)]
// pub struct TimeStamps {
//   sent: DateTime<Utc>,
//   recieved: Option<DateTime<Utc>>,
//   readby: Option<Vec<(Contact, DateTime<Utc>)>>,
// }

#[derive(Debug)]
pub enum ReceiptType {
  Delivered,
  Read,
}

#[derive(Debug, Clone)]
pub struct NotMyMessage {
  sender: Uuid,
  sent: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MyMessage {
  sent: DateTime<Utc>,
  delivered_to: Vec<Receipt>,
  read_by: Vec<Receipt>,
}

#[derive(Debug, Clone)]
pub enum Metadata {
  MyMessage(MyMessage),
  NotMyMessage(NotMyMessage),
}

#[derive(Debug, Clone)]
pub struct Reaction {
  emoji: char,
  author: Uuid,
}

#[derive(Debug, Clone)]
pub struct Message {
  body: MultiLineString,
  metadata: Metadata,
  quote: Option<u64>,
  reactions: Vec<Reaction>,
}

#[derive(Debug, Clone)]
pub struct Receipt {
  sender: Uuid,
  timestamp: DateTime<Utc>,
}

#[derive(Default, Debug)]
pub struct Location {
  index: usize,
  offset: i16,
  requested_scroll: i16,
}

// pub struct MyImageWrapper(StatefulProtocol);

// sshhhhhh
// impl Debug for MyImageWrapper {
//   fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
//     Ok(())
//   }
// }

#[derive(Hash, PartialEq, Eq, Debug)]
struct PhoneNumber(String);

impl Clone for PhoneNumber {
  fn clone(&self) -> Self {
    PhoneNumber(self.0.clone())
  }
}

#[derive(Debug, Default)]
struct MyGroup {
  name: String,
  // icon: Option<MyImageWrapper>,
  _description: String,
  num_members: usize,
}

// #[derive(Debug, Default)]
// pub struct Contact {
//   _name: String,
//   nick_name: String,
//   // pfp: Option<MyImageWrapper>,
//   // icon: Image,
// }

type Contacts = Arc<HashMap<Uuid, Profile>>;
type Groups = Arc<HashMap<GroupMasterKeyBytes, Group>>;

#[derive(Debug)]
struct MessageOptions {
  opened: bool,
  index: usize,
  timestamp: u64,
  mine: bool,
  // my_actions: Vec<Action>,
  // not_my_actions: Vec<Action>,
}

#[derive(Debug)]
pub struct Chat {
  thread: Thread,
  display: MyGroup,
  // a little convenience field so u dont have to get that hash map every time
  // thread: Thread
  messages: Vec<Message>,
  loaded_from: DateTime<Utc>,
  location: Location,
  message_options: MessageOptions,
  text_input: TextInput,
}

#[derive(Debug, Default, PartialEq)]
pub enum TextInputMode {
  #[default]
  Normal,
  Replying,
  Reacting,
  Editing,
}

#[derive(Debug, Default)]
pub struct TextInput {
  body: MultiLineString,
  cursor_index: u16,
  cursor_position: Position,
  mode: TextInputMode,
}

pub struct Settings {
  borders: bool,
  message_width_ratio: f32,
  _identity: String,
}

struct Account {
  name: String,
  username: String,
  number: PhoneNumber,
  uuid: Uuid,
}

impl Settings {
  fn init() -> Self {
    Self {
      borders: true,
      message_width_ratio: 0.8,
      _identity: "me".to_string(),
    }
  }
}

impl Model {
  fn init() -> Self {
    use uuid::uuid;

    let dummy_number = PhoneNumber("14124206767".to_string());
    let dummy_id = uuid!("00000000-0000-0000-0000-000000000000");

    // old code that im scared we still need
    // let messages = vec![
    //   Message {
    //     body: MultiLineString::new(
    //       "first message lets make this   message super looong jjafkldjaflk it was not long enough last time time to yap fr",
    //     ),
    //     metadata: Metadata::NotMyMessage(NotMyMessage {
    //       sender: dummy_id.clone(),
    //       sent: Utc::now().checked_sub_signed(TimeDelta::minutes(2)).expect("kaboom"),
    //     }),
    //   },
    //   Message {
    //     body: MultiLineString::new("second message"),
    //     metadata: Metadata::MyMessage(MyMessage {
    //       sent: Utc::now(),
    //       read_by: vec![(dummy_id.clone(), Some(Utc::now()))],
    //       delivered_to: vec![(dummy_id.clone(), None)],
    //     }),
    //   },
    //   Message {
    //     body: MultiLineString::new("a luxurious third message because im not convinced yet"),
    //     metadata: Metadata::MyMessage(MyMessage {
    //       sent: Utc::now(),
    //       read_by: vec![(dummy_id.clone(), None)],
    //       delivered_to: vec![(dummy_id.clone(), None)],
    //     }),
    //   },
    // ];
    //
    // let mut chat = Chat::default();
    //
    // for message in messages {
    //   chat.messages.push(message);
    // }

    // let picker = Picker::from_query_stdio().expect("kaboom");

    // Load an image with the image crate.
    // let dyn_img = image::ImageReader::open("./assets/ferris_the_wheel.jpg")
    //   .unwrap()
    //   .decode()
    //   .unwrap();

    // Create the Protocol which will be used by the widget.
    // let image = picker.new_resize_protocol(dyn_img.clone());
    // let image2 = picker.new_resize_protocol(dyn_img);

    // chat.participants = MyGroup {
    //   members: vec![dummy_id.clone()],
    //   name: "group 1".to_string(),
    //   // icon: Some(MyImageWrapper(image)),
    //   _description: "".to_string(),
    // };
    // chat.text_input = TextInput::default();
    // chat.location = Location {
    //   index: 1,
    //   offset: 0,
    //   requested_scroll: 0,
    // };
    // let chats: Vec<Chat> = vec![chat];

    let contacts = HashMap::new().into();

    // contacts.insert(
    //   dummy_id,
    //   Profile {
    //     name: Some(ProfileName {
    //       family_name: Some(String::from("nickname")),
    //       given_name: String::from("name"),
    //       // pfp: Some(MyImageWrapper(image2)),
    //     }),
    //     about: None,
    //     about_emoji: None,
    //     avatar: None,
    //     unrestricted_unidentified_access: true,
    //   },
    // );

    let account = Account {
      name: "non existant".to_string(),
      username: "not found".to_string(),
      number: dummy_number,
      uuid: dummy_id,
    };

    let model = Model {
      chat_index: 0,
      contacts: Arc::new(contacts),
      groups: Arc::new(HashMap::new()),
      chats: Vec::new().into(),
      account: account,
      running_state: RunningState::Running,
      mode: Arc::new(Mutex::new(Mode::Normal)),
      pinned_mode: Mode::Normal,
      // focus: Focus::Chats,
    };
    // let mut model = Model::default();
    // model.contacts = contacts;
    // // model.chats = chats;
    // model.chat_index = 0;
    model
  }

  // total hack for getting our own uuid and is not guarenteed to work
  // async fn find_self(&mut self, manager: &mut Manager<S, Registered>) -> Result<(), ()> {
  //   fn profiles_equal(profile1: &Profile, profile2: &Profile) -> bool {
  //     profile1.name == profile2.name && profile1.about == profile2.about
  //   }
  //
  //   let self_profile = match manager.retrieve_profile().await {
  //     Ok(x) => x,
  //     Err(_) => return Err(()),
  //   };
  //
  //   for (uuid, profile) in *self.contacts {
  //     let profile = match manager.retrieve_profile_by_uuid(uuid, profile.profile_key).await {
  //       Ok(x) => x,
  //       Err(_) => return Err(()),
  //     };
  //
  //     if profiles_equal(&self_profile, &profile) {
  //       self.account.uuid = uuid;
  //       return Ok(());
  //     }
  //   }
  //
  //   return Err(());
  // }

  // not really needed but it staves off the need for explicit liiftimes a little longer
  fn current_chat(&mut self) -> &mut Chat {
    &mut self.chats[self.chat_index]
  }

  fn find_chat(&mut self, thread: &Thread) -> Option<&mut Chat> {
    for chat in &mut self.chats {
      // maybe this rust thing isnt so bad (jk lol)
      if chat.thread == *thread {
        return Some(chat);
      }
    }
    // match thread {
    //   Thread::Contact(uuid) => {
    //     // Logger::log(format!(
    //     //   "thread: {}, with body: {}",
    //     //   uuid,
    //     //   message.body.clone().unwrap_or("useless message".to_string())
    //     // ));
    //
    //     Logger::log(format!("Could not find a chat that matched the id: {}", uuid));
    //   }
    //   _ => {}
    // }

    None
  }

  fn new_dm_chat(&mut self, profile: Profile, uuid: Uuid) {
    let chat = Chat::new(
      MyGroup {
        name: if let Some(name) = profile.name {
          name.given_name
        } else {
          "".to_string()
        },
        _description: if let Some(about) = profile.about {
          about
        } else {
          "".to_string()
        },
        num_members: 1,
      },
      Thread::Contact(uuid),
    );

    self.chats.push(chat);
  }
  pub fn new_group_chat(&mut self, group_key: GroupMasterKeyBytes, group: &Group) {
    let chat = Chat::new(
      MyGroup {
        name: group.title.clone(),
        _description: group.description.clone().unwrap_or("".to_string()),
        num_members: group.members.len(),
      },
      Thread::Group(group_key),
    );

    self.chats.push(chat);
  }
}

impl TextInput {
  fn render(
    &mut self,
    active: bool,
    message: Option<&Message>,
    contacts: &Contacts,
    area: Rect,
    buf: &mut Buffer,
  ) {
    let color = if active { Color::Magenta } else { Color::Reset };

    let mut block = Block::bordered()
      .border_set(border::THICK)
      .border_style(Style::default().fg(color));

    if self.mode == TextInputMode::Editing {
      block = block.title(Line::from(" Edit Message").left_aligned());
    }

    // minus 3 b/c you cant have the cursor on the border and i cant be bothered to add another
    // edge case
    let mut lines: Vec<Line> = Vec::new();

    if let Some(msg) = message {
      for line in msg.quote_lines(area.width as usize - 2, contacts) {
        lines.push(line);
      }
      // lines.push();
    }

    // logger.log(format!("this is the first line: {}", self.cursor_index));
    for yap in self.body.as_trimmed_lines(area.width - 3) {
      lines.push(Line::from(yap));
    }

    Paragraph::new(lines).block(block).render(area, buf);

    self.cursor_position = self.calc_cursor_position(area)
  }

  pub fn set_content(&mut self, string: String) {
    // dont be passing in no massive strings here now
    self.cursor_index = string.len() as u16;
    self.body.set_content(string);
  }

  fn calc_cursor_position(&mut self, area: Rect) -> Position {
    // gotta pad the border (still havent found a better way of doing this)
    let mut pos = Position {
      x: area.x + 1,
      y: area.y + 1,
    };
    // mad ugly calculations, smthns gotta change
    let lines = self.body.as_lines(area.width - 3);
    // let body = self.body.body.char_indices();

    let (mut index, mut row) = (0, 0);

    while (index + lines[row].len() as u16) < self.cursor_index {
      index += lines[row].len() as u16;
      pos.y += 1;
      row += 1;
    }

    pos.x += (self.cursor_index - index).clamp(0, area.width - 3);

    if self.mode == TextInputMode::Replying {
      pos.y += 3;
    }

    pos
  }

  fn insert_char(&mut self, char: char) {
    // some disgusting object-oriented blashphemy going on here
    self.body.insert(self.cursor_index as usize, char);
    self.cursor_index += 1;
  }

  fn delete_char(&mut self) {
    if self.cursor_index == 0 {
      return;
    }

    self.cursor_index -= 1;
    self.body.remove(self.cursor_index as usize);
  }

  fn clear(&mut self) {
    self.body.set_content("".to_string());
    self.cursor_index = 0;
  }
}

impl Metadata {
  fn new_mine(timestamp: DateTime<Utc>, members: usize) -> Self {
    let the_list = Vec::<Receipt>::with_capacity(members);

    // for uuid in recipients {
    //   the_list.push((*uuid, None));
    // }

    Self::MyMessage(MyMessage {
      sent: timestamp,
      delivered_to: the_list.clone(),
      read_by: the_list,
    })
  }

  // fn new_not_mine(timestamp: DateTime<Utc>, sender: Uuid) -> Self {
  //   Self::NotMyMessage(NotMyMessage {
  //     sent: timestamp,
  //     sender: sender,
  //   })
  // }
}

fn full_line<'a>(string: String, width: usize) -> Line<'a> {
  let mut padding = String::with_capacity(width);
  for _ in 0..(width - string.len()) {
    padding += " ";
  }

  Line::from(string + &padding)
}

impl MessageOptions {
  pub fn default() -> Self {
    Self {
      opened: false,
      // none of these fields rly mean anything if the box isnt open
      index: 0,
      mine: false,
      timestamp: 0,
    }
  }

  pub fn open(&mut self, ts: u64, mine: bool) {
    self.opened = true;
    self.index = 0;
    self.mine = mine;
    self.timestamp = ts;
  }

  // great method wow!
  pub fn close(&mut self) {
    self.opened = false;
  }

  pub fn select(&mut self) -> Action {
    let my_actions = vec![
      MessageOption::Reply,
      MessageOption::React,
      MessageOption::Edit,
      MessageOption::Copy,
      MessageOption::Info,
      MessageOption::Delete,
    ];
    let not_my_actions = vec![
      MessageOption::Reply,
      MessageOption::React,
      MessageOption::Copy,
      MessageOption::Info,
    ];
    if self.mine {
      Action::DoOption(my_actions[self.index])
    } else {
      Action::DoOption(not_my_actions[self.index])
    }
  }

  pub fn render(&self, message: &Message, area: Rect, buf: &mut Buffer) {
    // if !self.opened {
    //   return;
    // }
    let fixed_width = 16;

    let options = match message.metadata {
      Metadata::NotMyMessage(_) => {
        vec!["  Reply", "  React", "  Copy", "  Info"]
      }
      Metadata::MyMessage(_) => vec![
        "  Reply",
        "  React",
        "  Edit",
        "  Copy",
        "  Info",
        "  Delete",
      ],
    };
    let options: Vec<Vec<char>> = options.iter().map(|s| s.chars().collect()).collect();

    let length = options.len();

    let area = center_div(
      area,
      Constraint::Length(fixed_width),
      Constraint::Length(length as u16 + 2),
    );

    let mut lines = Vec::with_capacity(options.len());

    for (index, option) in options.into_iter().enumerate() {
      // this was so annoying oh man just let me index into a &str *I MADE* grrrrrr
      // let mut line = Line::from(vec![
      //   Span::from(option[0].to_string()).style(Style::default().bold()),
      //   Span::from((&option[1..]).iter().collect::<String>()),
      // ]);
      let mut line = Line::from(full_line(
        option.into_iter().collect::<String>(),
        fixed_width as usize,
      ));

      if index == self.index {
        line = line.style(Style::default().bg(Color::Magenta).fg(Color::Black));
      }

      lines.push(line);
    }

    // lines[self.index].style(Style::default().fg(Color::Magenta));
    let block = Block::bordered().border_set(border::THICK);

    Paragraph::new(lines)
      .block(block)
      .style(Style::default().bg(Color::Black))
      .render(area, buf);
  }
}

impl Message {
  fn render(
    &mut self,
    num_members: usize,
    active: bool,
    quoted: Option<FindMsgResult>,
    contacts: &Contacts,
    settings: &Settings,
    area: Rect,
    buf: &mut Buffer,
  ) {
    // should probably define this in settings
    let min_message_width = 15;

    // dont render anything if theres not enough space so we dont panic!
    if area.width < min_message_width {
      return;
    }

    let color = if active { Color::Magenta } else { Color::Reset };

    let mut block = Block::bordered()
      .border_set(border::THICK)
      .border_style(Style::default().fg(color));

    let mut displayed_metadata = Line::from(Span::from(self.format_duration()));

    let reactions = self.format_reactions();

    match &self.metadata {
      Metadata::NotMyMessage(meta) => {
        let name = match &contacts[&meta.sender].name {
          Some(name) => {
            name.given_name.clone()
            // match &name.family_name {
            //   Some(family_name) => family_name.clone(),
            //   None => name.given_name.clone(),
            // }
          }
          None => "smthns borken".to_string(),
        };
        block = block.title_top(Line::from(name).left_aligned());
        block = block.title_bottom(displayed_metadata.left_aligned());

        block = block.title_bottom(Line::from(reactions).right_aligned())
      }
      Metadata::MyMessage(_) => {
        displayed_metadata.push_span(Span::from(" "));
        displayed_metadata.push_span(self.format_delivered_status(num_members));

        block = block.title_bottom(displayed_metadata.right_aligned());

        block = block.title_bottom(Line::from(reactions).left_aligned())
      }
    }

    // im rly good at naming variables
    let mut my_area = area.clone();
    let availible_width = (area.width as f32 * settings.message_width_ratio + 0.5) as u16;
    my_area.width = availible_width;
    // let message_width: u16 = (area.width as f32 * settings.message_width_ratio + 0.5) as u16 - 2;

    let vec_lines: Vec<String> = self.body.as_trimmed_lines(my_area.width - 2);

    // shrink the message to fit if it does not need mutliple lines

    if vec_lines.len() == 1 {
      // these +- 2's are going to be the death of me
      my_area.width = cmp::max(vec_lines[0].len() as u16 + 2, min_message_width); // <----
      // ok listen ik this is bad but the only other way i could think of was to modify ^^
      // and that just seemed wrong ...
      if let Some(FindMsgResult::Found(msg)) = &quoted {
        my_area.width = cmp::max(
          my_area.width,
          cmp::min(availible_width, msg.body.body.len() as u16 + 2),
        );
      }
    }

    let mut lines: Vec<Line> = Vec::new();

    if self.quote.is_some() != quoted.is_some() {
      Logger::log("aint no way bruh");
    }

    if let Some(result) = quoted {
      let reply_lines = match result {
        FindMsgResult::Found(msg) => msg.quote_lines(my_area.width as usize - 2, contacts),
        FindMsgResult::NotLoaded => {
          // 29 is length of this "error" message
          my_area.width = cmp::max(cmp::min(29, availible_width), my_area.width);
          vec![
            Line::from("Message not loaded..."),
            Line::from("scroll up to see this message"),
            Line::from("-".repeat(my_area.width as usize - 2)),
          ]
        }
        FindMsgResult::NotExist => {
          my_area.width = cmp::max(cmp::min(30, availible_width), my_area.width);
          vec![
            Line::from("Message not found..."),
            Line::from("i suspect smthn has gone wrong"),
            Line::from("-".repeat(my_area.width as usize - 2)),
          ]
        }
      };
      for line in reply_lines {
        lines.push(line);
      }
    }

    for yap in vec_lines {
      lines.push(Line::from(yap));
    }
    // "allign" the chat to the right if it was sent by you
    // TODO: should add setting to toggle this behavior

    // shift message to the right if it is ur own
    if self.is_mine() {
      my_area.x += area.width - my_area.width;
    }

    // Logger::log(format!("drawing message here: {:?}", my_area));

    Paragraph::new(lines).block(block).render(my_area, buf)
    // .wrap(Wrap { trim: true })
  }

  fn upsert_reaction(&mut self, new_reaction: Reaction) {
    for reaction in &mut self.reactions {
      if reaction.author == new_reaction.author {
        reaction.emoji = new_reaction.emoji;
        return;
      }
    }

    self.reactions.push(new_reaction)
  }

  fn is_mine(&self) -> bool {
    if let Metadata::MyMessage(_) = self.metadata {
      true
    } else {
      false
    }
  }

  // really considering ditching chrono
  fn timestamp(&self) -> DateTime<Utc> {
    match &self.metadata {
      Metadata::NotMyMessage(x) => x.sent,
      Metadata::MyMessage(x) => x.sent,
    }
  }

  fn ts(&self) -> u64 {
    self.timestamp().timestamp_millis() as u64
  }

  // i thought i knew how lifetimes worked
  fn format_delivered_status<'explicit>(&self, num_members: usize) -> Span<'explicit> {
    let check_icon = " ";

    return match &self.metadata {
      Metadata::NotMyMessage(_) => Span::from(""),
      Metadata::MyMessage(x) => {
        if x.all_read(num_members) {
          Span::styled(
            [check_icon, check_icon].concat(),
            Style::default()
              .fg(Color::Magenta)
              .add_modifier(Modifier::BOLD),
          )
        } else if x.all_delivered(num_members) {
          Span::styled(
            [check_icon, check_icon].concat(),
            Style::default().fg(Color::Gray),
          )
        } else if x.sent() {
          Span::styled(check_icon, Style::default().fg(Color::Gray))
        } else {
          Span::styled("_", Style::default().fg(Color::White))
        }
      }
    };
  }

  pub fn format_duration(&self) -> String {
    let time = self.timestamp();
    let duration = Utc::now().signed_duration_since(time);

    if duration.num_minutes() < 1 {
      return "Now".to_string();
    } else if duration.num_hours() < 1 {
      let mut temp = duration.num_minutes().to_string();
      temp.push_str("m");
      return temp;
    } else if duration.num_days() < 1 {
      let mut temp = duration.num_hours().to_string();
      temp.push_str("h");
      return temp;
    } else {
      return time.format("%m/%d").to_string();
    }

    // let mut result = num.to_string();
    // result.push_str(chr);
    // result
  }

  pub fn quote_lines<'a, 'b>(&self, width: usize, contacts: &'a Contacts) -> Vec<Line<'a>> {
    let mut lines = vec![];

    lines.push(Line::from(vec![
      Span::from(match self.metadata {
        Metadata::MyMessage(_) => "You",
        Metadata::NotMyMessage(NotMyMessage { sender, .. }) => {
          if let Some(profile_name) = &contacts[&sender].name {
            &profile_name.given_name
          } else {
            "You found a bug!"
          }
        }
      }),
      Span::from(":"),
    ]));

    lines.push(Line::from(self.body.body.shrink(width)));
    lines.push(Line::from("-".repeat(width)));

    lines
  }

  fn format_reactions(&self) -> String {
    let mut output = String::with_capacity(self.reactions.len() * 2);
    for reaction in &self.reactions {
      output.push(reaction.emoji);
      output.push_str(" ");
    }

    output
  }

  fn height(&mut self, width: u16) -> u16 {
    let reply_height = if self.quote.is_some() { 3 } else { 0 };
    self.body.as_lines(width).len() as u16 + 2 + reply_height
  }
}

fn _format_vec(vec: &Vec<String>) -> String {
  let mut output = String::from("[");

  for thing in vec {
    output.push_str(thing);
    output.push_str(", ");
  }

  output.push_str("]");

  return output;
}

#[derive(Debug)]
pub enum FindMessageResult<'a> {
  NotLoaded,
  NotExist,
  Found(&'a mut Message),
}

// i am truly a programming powerhouse as seen by these lovely enums
#[derive(Debug)]
pub enum FindMsgResult {
  NotLoaded,
  NotExist,
  Found(Message),
}

// mhhhhh what a great "clone"
impl FindMessageResult<'_> {
  fn clone(self) -> FindMsgResult {
    match self {
      FindMessageResult::Found(msg) => FindMsgResult::Found(msg.clone()),
      FindMessageResult::NotLoaded => FindMsgResult::NotLoaded,
      FindMessageResult::NotExist => FindMsgResult::NotExist,
    }
  }
}

// pub type OptionOfMessage<'a> = Option<&'a mut Message>;
//
// impl<'a> Into<OptionOfMessage<'a>> for Result<&Message, FindMessageError> {
//   fn into(self) -> Option<&'a mut Message> {
//     match self {
//       Ok(msg) => Some(msg),
//       Err(_) => None,
//     }
//   }
// }

impl Chat {
  fn new(display: MyGroup, thread: Thread) -> Self {
    Chat {
      thread: thread,
      display,
      messages: Vec::new(),
      loaded_from: Utc::now(),
      text_input: TextInput::default(),
      location: Location::zero(),
      message_options: MessageOptions::default(),
    }
  }

  fn render(
    &mut self,
    area: Rect,
    buf: &mut Buffer,
    settings: &Settings,
    contacts: Contacts,
    mode: Mode,
  ) {
    let input_lines = self.text_input.body.rows(area.width - 3);
    // Logger::log("this is our input: ".to_string());
    // Logger::log(format_vec(self.text_input.body.as_lines(area.width - 2)));

    let mut reply_lines = 0;

    let reply_message = if self.text_input.mode == TextInputMode::Replying {
      reply_lines = 3;
      // rly painful clone here but whatever
      match self.find_message(self.message_options.timestamp) {
        Some(msg) => Some(&msg.clone()),
        None => None,
      }
    } else {
      None
    };

    let layout = Layout::vertical([
      Constraint::Min(6),
      Constraint::Length(input_lines + reply_lines + 2),
    ])
    .split(area);

    self.text_input.render(
      mode == Mode::Insert,
      reply_message,
      &contacts,
      layout[1],
      buf,
    );

    // kind of a sketchy shadow here but the layout[1] is used like once
    let area = layout[0];

    let block = Block::bordered().border_set(border::THICK);
    // .title(title.centered())
    // .title_bottom(instructions.centered())
    block.render(area, buf);

    if self.messages.len() == 0 {
      return;
    }

    // shitty temp padding for the border
    let mut area = area;
    area.x += 1;
    area.width -= 2;
    area.height -= 2;
    area.y += 1;
    // end shitty tmp padding

    let message_width: u16 = (area.width as f32 * settings.message_width_ratio + 0.5) as u16 - 2;

    let mut scroll = self.location.requested_scroll;
    let mut index = self.location.index;
    let mut offset = self.location.offset;

    // yeah this scrolling logic is a little ugly but im not sure how to make it less so
    // also im a little scared to touch it
    if scroll > 0 {
      while scroll > 0 {
        if index + 1 == self.messages.len() {
          offset = 0;
          break;
        }

        let height = self.messages[index + 1].height(message_width);

        if height as i16 > scroll + offset {
          offset += scroll;
          break;
        }
        index += 1;
        scroll -= height as i16;

        if scroll < 0 {
          offset += scroll;
          scroll = 0;
        }
      }
    } else if scroll < 0 {
      while scroll < 0 {
        if offset as i16 >= scroll * -1 {
          offset += scroll;
          break;
        }
        if index == 0 {
          offset = 0;
          break;
        }

        let height = self.messages[index].height(message_width);
        scroll += height as i16;
        index -= 1;

        if scroll > 0 {
          offset = scroll;
          scroll = 0;
        }
      }
    }

    self.location.index = index;
    self.location.offset = offset;
    self.location.requested_scroll = 0;

    let mut y = area.height as i16 - self.location.offset;

    loop {
      // "rust is a good programing language"
      let message = &mut self.messages[index];

      let quoted = if let Some(timestamp) = message.quote {
        Some(self.find_message_result(timestamp).clone())
      } else {
        None
      };

      // "rust is a good programing language"
      let message = &mut self.messages[index];

      let height = message.height(message_width);

      y -= height as i16;
      if y < 0 {
        break;
      }

      // let height = min(y + requested_height, area.height);
      let new_area = Rect::new(area.x, area.y + y as u16, area.width, height as u16);

      message.render(
        self.display.num_members,
        self.location.index == index && mode == Mode::Normal,
        quoted,
        &contacts,
        settings,
        new_area,
        buf,
      );

      if index == 0 {
        break;
      }

      index -= 1;
    }

    if mode == Mode::MessageOptions {
      self
        .message_options
        .render(&self.messages[self.location.index], area, buf);
    }
  }

  fn last_message(&self) -> Option<&Message> {
    let last = self.messages.len();
    if last <= 0 {
      None
    } else {
      Some(&self.messages[last - 1])
    }
  }

  fn last_message_mut(&mut self) -> Option<&mut Message> {
    let last = self.messages.len();
    if last <= 0 {
      None
    } else {
      Some(&mut self.messages[last - 1])
    }
  }

  fn selected_message(&self) -> Option<&Message> {
    if self.messages.len() > 0 {
      Some(&self.messages[self.location.index])
    } else {
      None
    }
  }

  fn insert_message(&mut self, message: Message) {
    // let new_timestamp = message.timestamp();

    let timestamp = message.ts();
    let mut i = self.messages.len();

    while i > 0 {
      // Logger::log(format!("old timestamp: {} -- new timestamp: {}", ts, timestamp));

      let ts = self.messages[i - 1].ts();

      if timestamp > ts {
        break;
      }

      if timestamp == ts {
        return;
      }

      i -= 1;
    }

    // let metadata = if mine {
    //   Metadata::new_mine(
    //     DateTime::from_timestamp_millis(timestamp as i64).expect("kaboom"),
    //     &self.participants.members,
    //   )
    // } else {
    //   Metadata::new_not_mine(
    //     DateTime::from_timestamp_millis(timestamp as i64).expect("kaboom"),
    //     sender,
    //   )
    // };

    // let body = match &message {
    //   DataMessage { body: Some(body), .. } => body,
    //   // if there isnt a body its an attachment that we cant display
    //   _ => return (),
    //   // _ => "Attachment that we cant display yet".to_string().clone(),
    // };

    // let parsed_message = Message {
    //   body: MultiLineString::new(body),
    //   metadata: meta,
    //   quote: None,
    // };

    self.messages.insert(i, message);

    if self.messages.len() == 1 {
      return;
    }

    // mhhhh yes what a wonderful state machine i have created
    // i am sure this will break in a grand total of *ZERO* ways
    if !self.message_options.opened {
      // little bit of a goofy statement to not underflow a usize
      if self.messages.len() - 1 == self.location.index + 1 || i <= self.location.index {
        // Oh noooooo, i have violated the ELM design patterns ....
        // however we will go on with our days ... ?
        self.location.index += 1;
      }
    }
  }

  fn upsert_reaction(&mut self, reaction: Reaction, target_timestamp: u64) {
    if let Some(message) = self.find_message(target_timestamp) {
      message.upsert_reaction(reaction)
    }
  }

  // yeah this code is cleary repeated, but like the return types would have to be rly ugly if i
  // resued the same function sooooo ....
  fn index(&self, timestamp: u64) -> Option<usize> {
    let mut i = self.messages.len();

    if i == 0 {
      return None;
    }

    while i > 0 {
      i -= 1;

      let ts = self.messages[i].ts();

      if timestamp > ts {
        break;
      }

      if timestamp == ts {
        return Some(i);
      }
    }

    None
  }

  fn find_message_result(&mut self, timestamp: u64) -> FindMessageResult<'_> {
    // Logger::log(format!("looking for: {}", timestamp));
    let mut i = self.messages.len();

    if i == 0 {
      return FindMessageResult::NotLoaded;
    }

    while i > 0 {
      // Logger::log(format!("old timestamp: {} -- new timestamp: {}", ts, timestamp));
      i -= 1;

      let ts = self.messages[i].ts();
      // Logger::log(format!("checking: {}", ts));

      if timestamp > ts {
        // Logger::log(format!(
        //   "could not find message at time: {:?} in thread: {:#?}",
        //   timestamp, self.thread
        // ));
        return FindMessageResult::NotExist;
      }

      if timestamp == ts {
        return FindMessageResult::Found(&mut self.messages[i]);
      }
    }

    FindMessageResult::NotLoaded
  }

  fn find_message(&mut self, timestamp: u64) -> Option<&mut Message> {
    // Logger::log(format!("looking for: {}", timestamp));
    match self.find_message_result(timestamp) {
      FindMessageResult::Found(msg) => Some(msg),
      _ => None,
    }
  }

  fn delete_message(&mut self, timestamp: u64) {
    let index = self
      .index(timestamp)
      .expect("could not find message to delete");
    self.messages.remove(index);

    if self.location.index >= index {
      self.location.index -= 1;
    }
  }

  // fn add_receipt(&mut self, receipt: Receipt) {
  //   let timestamp = receipt.timestamp.timestamp_millis() as u64;
  //
  //   // can panick here must fix
  //   let mut i = self.messages.len() - 1;
  //
  //   // the borrow checker rly did not want me doing this
  //   match &mut self.messages[i].metadata {
  //     &mut Metadata::MyMessage(MyMessage {
  //       read_by: ref mut read_by,
  //       delivered_to: ref mut delivered_to,
  //       ..
  //     }) => {
  //       read_by.push(receipt.clone());
  //
  //       delivered_to.push(receipt);
  //     }
  //     _ => {}
  //   }
  // }
  //
  fn load_more_messages(&mut self, spawner: &SignalSpawner, delta: TimeDelta) {
    self.loaded_from = self.loaded_from.checked_sub_signed(delta).unwrap();

    let (uuid, group_key) = match &self.thread {
      Thread::Contact(uuid) => (Some(uuid.clone()), None),
      Thread::Group(group_key) => (None, Some(group_key.clone())),
    };

    spawner.spawn(Cmd::ListMessages {
      // not scuffed at all
      recipient_uuid: uuid,
      group_master_key: group_key,
      from: Some(self.loaded_from.timestamp_millis() as u64),
    });
  }

  fn send(&mut self, spawner: &SignalSpawner) {
    Logger::log("sending a message".to_string());
    // slight optimization possible here
    let data = self.text_input.body.body.clone();

    // let members = self.participants.members.clone();

    let ts = Utc::now();

    let mut quote_stamp = None;
    let mut quote = None;

    match self.text_input.mode {
      TextInputMode::Normal | TextInputMode::Replying => {
        if self.text_input.mode == TextInputMode::Replying {
          quote_stamp = Some(self.message_options.timestamp);
          quote = Some(get_quote(self.find_message(quote_stamp.unwrap()).unwrap()));
        }

        spawner.spawn(Cmd::SendToThread {
          thread: self.thread.clone(),
          message: data,
          quote: quote,
          timestamp: ts.timestamp_millis() as u64,
          attachment_filepath: Vec::new().into(),
        });

        // maybe i should implement this by returning an Action enum but i cant be bothered rn
        //
        // maybe i should also use the function i already have for adding messages, but thats designed
        // for DATA messages

        self.messages.push(Message {
          body: MultiLineString::new(&self.text_input.body.body),
          // this now timestamp is a little sketchy cuz the server is the one who actually says when
          // what happened
          metadata: Metadata::new_mine(ts, self.display.num_members),
          quote: quote_stamp,
          reactions: vec![],
        });

        // scroll down if we r at the bottom (this logic is def repeated and shouldnt be)
        if self.messages.len() - 1 == self.location.index + 1 {
          // Oh noooooo, i have violated the ELM design patterns ....
          // however we will go on with our days ... ?
          self.location.index += 1;
        }
      }
      TextInputMode::Reacting => {
        let ts = self.message_options.timestamp;
        // super readable !!
        let uuid = if let Some(Message {
          metadata: Metadata::NotMyMessage(NotMyMessage { sender, .. }),
          ..
        }) = self.find_message(ts)
        {
          Some(sender.clone())
        } else {
          None
        };

        spawner.spawn(Cmd::ReactToThread {
          thread: self.thread.clone(),
          reaction: data.clone(),
          timestamp: Utc::now().timestamp_millis() as u64,
          target_timestamp: ts,
          author_uuid: uuid,
        });

        self
          .find_message(ts)
          .expect("no way these come back to bite me")
          .upsert_reaction(Reaction {
            emoji: data.chars().nth(0).unwrap(),
            author: spawner.self_uuid,
          });
      }
      TextInputMode::Editing => {
        let target_message = self.find_message(self.message_options.timestamp).unwrap();
        target_message.body.set_content(data.clone());

        let target_timestamp = target_message.ts();

        spawner.spawn(Cmd::EditMessage {
          thread: self.thread.clone(),
          message: data,
          timestamp: ts.timestamp_millis() as u64,
          target_timestamp: target_timestamp,
        });
      }
    }

    self.text_input.clear();
    self.text_input.mode = TextInputMode::Normal;
  }
}

impl Location {
  fn zero() -> Self {
    Self {
      index: 0,
      offset: 0,
      requested_scroll: 0,
    }
  }
}

trait MyStringUtils {
  fn shrink<T>(&self, width: T) -> String
  where
    T: Into<usize>;

  fn byte_index<Int>(&self, char_idx: Int) -> usize
  where
    Int: Into<usize>;
}

impl MyStringUtils for String {
  fn shrink<T>(&self, width: T) -> String
  where
    T: Into<usize>,
  {
    let width = width.into();

    let mut fitted = self.clone();

    if fitted.len() <= width {
      return fitted;
    } else {
      fitted = fitted[..width - 3].to_string();
      fitted.push_str("...");
      // fitted.push("...");
      return fitted;
    }
  }

  // some rare llm code
  fn byte_index<Int>(&self, char_idx: Int) -> usize
  where
    Int: Into<usize>,
  {
    self
      .char_indices()
      .nth(char_idx.into())
      .map(|(i, _)| i)
      .unwrap_or_else(|| self.len())
  }
}

fn format_duration_fancy(time: &DateTime<Utc>) -> String {
  let duration = Utc::now().signed_duration_since(time);

  if duration.num_minutes() < 1 {
    return "Now".to_string();
  } else if duration.num_hours() < 1 {
    let mut temp = duration.num_minutes().to_string();
    temp.push_str(" minutes ago");
    return temp;
  } else if duration.num_days() < 1 {
    let mut temp = duration.num_hours().to_string();
    temp.push_str(" hours ago");
    return temp;
  } else {
    return time.format("%m/%d").to_string();
  }
}

impl MyMessage {
  fn all_read(&self, num_members: usize) -> bool {
    self.read_by.len() == num_members
  }

  fn all_delivered(&self, num_members: usize) -> bool {
    self.delivered_to.len() == num_members
  }

  // what a great function i totall have not forgot about
  fn sent(&self) -> bool {
    true
  }
}

fn render_group(chat: &mut Chat, active: bool, hovered: bool, area: Rect, buf: &mut Buffer) {
  // Logger::log(format!("{}", active));
  // let icon = &mut chat.participants.icon;
  //
  // Block::bordered().border_set(border::THICK).render(area, buf);
  //

  let color = if active {
    if hovered { Color::Magenta } else { Color::Gray }
  } else {
    Color::Black
  };

  let area = pad_with_border(color, area, buf);

  let layout = Layout::horizontal([
    Constraint::Length(7),
    Constraint::Min(15),
    Constraint::Length(6),
  ])
  .split(area);

  // let image = StatefulImage::default().resize(Resize::Crop(None));
  // let mut pfp = match &self.pfp {
  //   Some(x) => x.0,
  //   None => panic!("Aaaaaahhhhh"),
  // };
  // // StatefulImage::render(image, layout[0], buf, &mut pfp);
  // let image: StatefulImage<StatefulProtocol> = StatefulImage::default();

  // match icon.as_mut() {
  // Some(image) => StatefulImage::new().render(area, buf, &mut image.0),
  // None => {}
  // }
  let group = &chat.display;
  let mut innner_lines: Vec<Line> = vec![Line::from(group.name.shrink(layout[1].width).bold())];

  // display the last message sent in the chat if there was one (there usually will be one)
  if let Some(last_message) = chat.last_message() {
    let message_text: Vec<String> = last_message.body.fit(layout[1].width, layout[1].height - 1);

    for line in message_text {
      innner_lines.push(Line::from(line));
    }

    let time = last_message.format_duration();

    Paragraph::new(vec![
      Line::from(time),
      Line::from(last_message.format_delivered_status(chat.display.num_members)),
    ])
    .render(layout[2], buf);
  }

  Paragraph::new(innner_lines).render(layout[1], buf);
}

fn one_by_two_area(x: u16, y: u16) -> Rect {
  Rect {
    x: x,
    y: y,
    width: 2,
    height: 1,
  }
}

fn render_qr(qr: QrCode, mut area: Rect, buf: &mut Buffer) {
  let block = "██";
  let pad_style = Style::default().fg(Color::White);

  for y in 0..area.height {
    Span::style(block.repeat(area.width as usize).into(), pad_style).render(
      Rect {
        x: area.x,
        y: area.y + y,
        width: area.width,
        height: 1,
      },
      buf,
    );
  }

  area.x += 2;
  area.y += 1;

  for y in 0..qr.size() {
    for x in 0..qr.size() {
      Span::styled(
        block,
        Style::default().fg(match qr.get_module(x, y) {
          true => Color::Black,
          false => Color::White,
        }),
      )
      .render(
        one_by_two_area(area.x + 2 * x as u16, area.y + y as u16),
        buf,
      );
      // (... paint qr.get_module(x, y) ...)
    }
  }
}

fn draw_linking_screen(state: &LinkState, frame: &mut Frame) {
  let area = frame.area();
  let buf = frame.buffer_mut();

  let mut area = pad_with_border(Color::White, area, buf);

  let mut size: u16 = 1;

  match &state.url {
    Some(url) => {
      let qr = QrCode::encode_text(&url.to_string(), QrCodeEcc::Medium);

      match qr {
        Ok(qr) => {
          size = qr.size() as u16 + 2;
          let qr_area = center_div(area, Constraint::Length(size * 2), Constraint::Length(size));

          let needed_size = qr.size() as u16 + 2;

          // dont render qr-code if we dont have enough space
          if needed_size + 3 > area.height || needed_size * 2 > area.width {
            Line::from("Window too small to render qr code")
              .style(Style::default().fg(Color::Red))
              .centered()
              .render(qr_area, buf);
            return;
          }

          let mut instruction_area = qr_area.clone();
          instruction_area.y -= 1;
          Line::from("scan the qr-code to link your account")
            .centered()
            .render(instruction_area, buf);
          area.x = qr_area.x;
          area.width = qr_area.width;
          area.y = qr_area.height + qr_area.y;
          area.height = area.height - area.y;

          render_qr(qr, qr_area, buf);
        }

        Err(_) => Line::from("Error generating qrcode (tough shit pal)").render(area, buf),
      }

      let raw_url = vec![
        Line::from("Or visit the url like a caveman:").centered(),
        Line::from(url.to_string()),
      ];
      Paragraph::new(raw_url).render(
        area, // Rect {
        //   x: area.x,
        //   y: area. y,
        //   width: area.width,
        //   height: area.height - size,
        // },
        buf,
      );
    }

    None => Line::from("Generating Linking Url ...").render(area, buf),
  }
}

fn center_div(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
  let [area] = Layout::horizontal([horizontal])
    .flex(Flex::Center)
    .areas(area);
  let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
  area
}

fn center_vertical(area: Rect, height: u16) -> Rect {
  let [area] = Layout::vertical([Constraint::Length(height)])
    .flex(Flex::Center)
    .areas(area);
  area
}

fn draw_loading_sreen(state: &LoadState, frame: &mut Frame) {
  let area = frame.area();
  let buf = frame.buffer_mut();

  let mut area = pad_with_border(Color::White, area, buf);

  area.y += 1;

  // let fist_loaded = Utc::now().signed_duration_since(state.first_timestamp);
  // let last_laoded = Utc::now().signed_duration_since(state.latest_timestamp);

  // this shouldnt happen basically ever but its a weird edge case
  // if state.raw_duration == None || state.latest_timestamp == None {
  //   return;
  // }

  // these should only happen like immediately on start up
  if let Some(raw_duration) = state.raw_duration {
    if let Some(latest_timestamp) = state.latest_timestamp {
      let formatted_duration =
        format_duration_fancy(&DateTime::from_timestamp_millis(latest_timestamp as i64).unwrap());

      let partial_duration = Utc::now().timestamp_millis() as u64 - latest_timestamp;

      let percent = 1.0 as f64 - (partial_duration as f64 / raw_duration as f64);

      // TODO: fiddle with this stuff a little
      let area = center_div(area, Constraint::Length(40), Constraint::Percentage(20));

      let mut area = pad_with_border(Color::White, area, buf);

      Line::from(["Loading messages from ", &formatted_duration].concat())
        .centered()
        .render(area, buf);

      area.y += 1;

      let area = center_vertical(area, 2);

      Gauge::default()
        .gauge_style(Style::new().white().on_black().italic())
        .ratio(percent.clamp(0.0, 1.0))
        .render(area, buf);
    }
  } else {
    Line::from("Loading past messages ...").render(area, buf);
  }
}

// main ---
async fn real_main() -> anyhow::Result<()> {
  _ = Logger::init("log.txt");
  // regular lumber jack
  Logger::log("testing".to_string());

  // tui::install_panic_hook();
  let mut terminal = ratatui::init();
  let (action_tx, mut action_rx) = mpsc::unbounded_channel();

  // let mode = Arc::clone(&model.mode);
  let mode = Arc::new(Mutex::new(Mode::default()));

  let cloned_mode = Arc::clone(&mode);
  let action_tx1 = action_tx.clone();
  let updater = tokio::spawn(async move {
    handle_crossterm_events(action_tx1, &cloned_mode).await;
  });

  let db_path = default_db_path();

  Logger::log(&db_path);
  // let db_path = "/home/mqngo/Coding/rust/signal-tui/plzwork.db3";
  let mut config_store =
    SqliteStore::open_with_passphrase(&db_path, "secret".into(), OnNewIdentity::Trust).await?;

  // tokio::spawn(run(
  //   Cmd::LinkDevice {
  //     servers: SignalServers::Production,
  //     device_name: "terminal enjoyer".to_string(),
  //   },
  //   config_store.clone(),
  //   action_tx.clone(),
  // ));

  // link device if not already
  if !config_store.is_registered().await {
    let mut linking_model = LinkState { url: None };

    link_device(
      SignalServers::Production,
      "terminal enjoyer".to_string(),
      action_tx.clone(),
    );

    // spawner.spawn(Cmd::LinkDevice {
    //   servers: SignalServers::Production,
    //   device_name: "terminal enjoyer".to_string(),
    // });
    //

    loop {
      terminal.draw(|f| draw_linking_screen(&linking_model, f))?;

      // Handle events and map to a Message
      let current_msg = action_rx.recv().await;

      match current_msg {
        Some(Action::Link(linking)) => match linking {
          LinkingAction::Url(url) => linking_model.url = Some(url),
          LinkingAction::Success => break,
          LinkingAction::Fail => link_device(
            SignalServers::Production,
            "terminal enjoyer".to_string(),
            action_tx.clone(),
          ),
          //   spawner.spawn(Cmd::LinkDevice {
          //   servers: SignalServers::Production,
          //   device_name: "terminal enjoyer".to_string(),
          // }),
        },

        Some(Action::Quit) => {
          return Ok(());
        }

        Some(_) => {}

        None => {
          Logger::log("I dont think this should ever happenn".to_string());
        }
      }
    }

    // there probably a better way to make the store linked but this only happens once so idc
    config_store =
      SqliteStore::open_with_passphrase(&db_path, "secret".into(), OnNewIdentity::Trust).await?;
  }

  // initialize all the important stuff
  let manager = Manager::load_registered(config_store)
    .await
    .expect("why even try anymore?");

  let mut model = Model::init();
  model.mode = Arc::clone(&mode);
  model.account.uuid = manager.registration_data().service_ids.aci;
  Logger::log(format!(
    "well isnt this convenient: {:#?}",
    &model.account.uuid
  ));

  let settings = &Settings::init();

  let spawner = SignalSpawner::new(manager, action_tx.clone());

  // spawner.sync_contacts();
  _ = update_contacts(&mut model, &spawner).await;
  _ = model.update_groups(&spawner).await;

  // receive all past messages

  let mut loading_model = LoadState {
    raw_duration: None,
    latest_timestamp: None,
  };

  Logger::log("about to draw the first loading screen".to_string());

  loop {
    terminal.draw(|f| draw_loading_sreen(&loading_model, f))?;

    let msg = action_rx.recv().await;

    // this whole thing is really ugly, im basically stuffing all the parts of TEA into this loop,
    // while also calling the normal update function for the main model
    match msg {
      Some(Action::Receive(ref receive)) => {
        match receive {
          Received::QueueEmpty => break,
          Received::Contacts => Logger::log("we gyatt some contacts".to_string()),
          Received::Content(content) => {
            match loading_model.raw_duration {
              None => {
                loading_model.raw_duration =
                  Some(Utc::now().timestamp_millis() as u64 - content.metadata.timestamp)
              }
              _ => {}
            }

            loading_model.latest_timestamp = Some(content.metadata.timestamp);
          }
        }

        update(
          &mut model,
          msg.expect("the laws of physics have collapsed"),
          &spawner,
        )
        .await;
      }

      Some(Action::Quit) => {
        return Ok(());
      }

      Some(_) => {}

      None => {
        Logger::log("I dont think this should ever happenn".to_string());
      }
    }
  }

  // action_tx.send(Action::Receive(Received::Contacts));

  let mut stdout = io::stdout();

  // load some initial messages just in case
  for chat in &mut model.chats {
    if chat.messages.len() < 1 {
      chat.load_more_messages(&spawner, TimeDelta::try_hours(2).unwrap());
    }
  }

  while model.running_state != RunningState::OhShit {
    // Render the current view
    terminal.draw(|frame| view(&mut model, frame, &mut stdout, settings))?;

    // Handle events and map to a Message
    let mut current_msg = action_rx.recv().await;

    // Process updates as long as they return a non-None message
    while current_msg.is_some() {
      current_msg = update(&mut model, current_msg.unwrap(), &spawner).await;
    }
  }

  updater.abort();
  // updater.await.unwrap_err();
  Ok(())
}

// main ---
#[allow(unexpected_cfgs)]
#[tokio::main(flavor = "local")]
async fn main() {
  let result = real_main().await;

  ratatui::restore();

  match result {
    Ok(_) => Logger::log(format!("we are a-okay")),
    Err(_) => {
      Logger::log("we are NYAT a-okay");
      Logger::log(format!("{:?}", result));
    }
  }
}

// an expirmental way to make the borrow checker less mad at me constantly (currently not a fan of
// it though)
// fn render_chat(model: &mut Model, contact: &Contacts, settings: &Settings, mode: &Mode, area: Area, buf: &mut Buffer) {
//   let chat = model.current_chat();
//
//   let input_lines = chat.text_input.body.rows(area.width - 3);
//   // Logger::log("this is our input: ".to_string());
//   // Logger::log(format_vec(chat.text_input.body.as_lines(area.width - 2)));
//
//   let layout = Layout::vertical([Constraint::Min(6), Constraint::Length(input_lines + 2)]).split(area);
//
//   chat./ext_input.render(layout[1], buf);
//
//   // kind of a sketchy shadow here but the layout[1] is used like once
//   let area = layout[0];
//
//   let block = Block::bordered().border_set(border::THICK);
//   // .title(title.centered())
//   // .title_bottom(instructions.centered())
//   block.render(area, buf);
//
//   if chat.messages.len() == 0 {
//     return;
//   }
//
//   // shitty temp padding for the border
//   let mut area = area;
//   area.x += 1;
//   area.width -= 2;
//   area.height -= 2;
//   area.y += 1;
//   // end shitty tmp padding
//
//   let message_width: u16 = (area.width as f32 * settings.message_width_ratio + 0.5) as u16 - 2;
//
//   let mut scroll = chat.location.requested_scroll;
//   let mut index = chat.location.index;
//   let mut offset = chat.location.offset;
//
//   // yeah this scrolling logic is a little ugly but im not sure how to make it less so
//   // also im a little scared to touch it
//   if scroll > 0 {
//     while scroll > 0 {
//       if index + 1 == chat.messages.len() {
//         offset = 0;
//         break;
//       }
//
//       let height = chat.messages[index + 1].height(message_width);
//
//       if height as i16 > scroll + offset {
//         offset += scroll;
//         break;
//       }
//       index += 1;
//       scroll -= height as i16;
//
//       if scroll < 0 {
//         offset += scroll;
//         scroll = 0;
//       }
//     }
//   } else if scroll < 0 {
//     while scroll < 0 {
//       if offset as i16 >= scroll * -1 {
//         offset += scroll;
//         break;
//       }
//       if index == 0 {
//         offset = 0;
//         break;
//       }
//
//       let height = chat.messages[index].height(message_width);
//       scroll += height as i16;
//       index -= 1;
//
//       if scroll > 0 {
//         offset = scroll;
//         scroll = 0;
//       }
//     }
//   }
//
//   chat.location.index = index;
//   chat.location.offset = offset;
//   chat.location.requested_scroll = 0;
//
//   let mut y = area.height as i16 - chat.location.offset;
//
//   loop {
//     let message = &mut self.messages[index];
//
//     let height = message.body.rows(message_width) + 2;
//
//     y -= height as i16;
//     if y < 0 {
//       break;
//     }
//
//     // let height = min(y + requested_height, area.height);
//     let new_area = Rect::new(area.x, area.y + y as u16, area.width, height as u16);
//
//     message.render(new_area, buf, settings, &contacts, self.location.index == index);
//
//     if index == 0 {
//       break;
//     }
//
//     index -= 1;
//   }
// }

fn view(model: &mut Model, frame: &mut Frame, stdout: &mut Stdout, settings: &Settings) {
  let title = Line::from(" Counter App Tutorial ".bold());
  let instructions = Line::from(vec![
    " Decrement ".into(),
    "<Left>".blue().bold(),
    " Increment ".into(),
    "<Right>".blue().bold(),
    " Quit ".into(),
    "<Q> ".blue().bold(),
  ]);
  let _block = Block::bordered()
    .title(title.centered())
    .title_bottom(instructions.centered())
    .border_set(border::THICK);

  // let _counter_text = Text::from(vec![Line::from(vec![
  //   "Value: ".into(),
  //   model.counter.to_string().yellow(),
  // ])]);

  let layout = Layout::new(
    Direction::Horizontal,
    vec![Constraint::Percentage(40), Constraint::Percentage(60)],
  )
  .split(frame.area());

  _ = Block::bordered()
    .border_set(border::THICK)
    .render(layout[0], frame.buffer_mut());

  let contact_height = 3 + 2;

  let mut contact_area = layout[0];
  contact_area.width -= 2;
  contact_area.height = contact_height;
  contact_area.x += 1;
  contact_area.y += 1;

  let mut index = 0;

  while contact_area.y < layout[0].height && index < model.chats.len() {
    let chat = &mut model.chats[index];
    render_group(
      chat,
      index == model.chat_index,
      model.pinned_mode == Mode::Groups,
      contact_area,
      frame.buffer_mut(),
    );
    // let last = &(&mut model.chats)[index].last_message();
    // model.chats[index].participants.render(last, contact_area, frame.buffer_mut());
    contact_area.y += contact_height;
    index += 1;
  }

  // wow im good at coding
  let contacts = Arc::clone(&model.contacts);
  let mode = model.pinned_mode.clone();

  match model.pinned_mode {
    Mode::Insert | Mode::Normal | Mode::Groups | Mode::MessageOptions => {
      // render_chat(
      //   model,
      //   contacts,
      //   settings,
      //   model.pinned_mode,
      //   layout[1],
      //   frame.buffer_mut(),
      // );
      model
        .current_chat()
        .render(layout[1], frame.buffer_mut(), settings, contacts, mode);

      frame.set_cursor_position(model.current_chat().text_input.cursor_position);

      // raw dawging some stdio
      // (also i sense this representation of a "mode" is going to collapse in the near future ... )
      if model.pinned_mode == Mode::Insert {
        _ = stdout.execute(cursor::SetCursorStyle::SteadyBar);
      } else {
        _ = stdout.execute(cursor::SetCursorStyle::SteadyBlock);
      }
    }
    Mode::Settings => {
      render_settings(layout[1], frame.buffer_mut(), settings, &model.account);
    } // _ => {}
  }
}

fn pad_with_border(color: Color, area: Rect, buf: &mut Buffer) -> Rect {
  Block::bordered()
    .border_set(border::THICK)
    .border_style(Style::default().fg(color))
    .render(area, buf);

  Rect {
    x: area.x + 1,
    y: area.y + 1,
    width: area.width - 2,
    height: area.height - 2,
  }

  // area.x += 1;
  // area.y += 1;
  // area.width -= 2;
  // area.height -= 2;
  // area
}

// what a goofy function i dont want to fix anytime soon
fn render_settings(area: Rect, buf: &mut Buffer, _settings: &Settings, account: &Account) {
  let area = pad_with_border(Color::Reset, area, buf);

  let info = vec![
    Line::from("Name: ".to_string() + &account.name),
    Line::from("Username: ".to_string() + &account.username),
    Line::from("Number: ".to_string() + &account.number.0),
  ];

  Paragraph::new(info).render(area, buf);
}
