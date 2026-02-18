use std::cmp::min;

use crate::{MyStringUtils, RatatuiBodyRange};
use ratatui::{
  style::Style,
  text::{Line, Span},
};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct MySpan {
  style: Style,
  content: String,
}

impl From<&str> for MySpan {
  fn from(value: &str) -> Self {
    Self {
      style: Style::default(),
      content: value.to_string(),
    }
  }
}

impl From<String> for MySpan {
  fn from(value: String) -> Self {
    Self {
      style: Style::default(),
      content: value,
    }
  }
}

impl Into<Span<'_>> for &MySpan {
  fn into(self) -> Span<'static> {
    Span {
      style: self.style,
      content: self.content.clone().into(),
    }
  }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CharSpan {
  style: Style,
  char: char,
}

impl From<char> for CharSpan {
  fn from(value: char) -> Self {
    Self {
      style: Style::default(),
      char: value,
    }
  }
}
// impl From<String> for CharSpan {
//   fn from(value: String) -> Self {
//     value.chars().nth(0).unwrap().into()
//   }
// }

impl Into<Span<'_>> for &CharSpan {
  fn into(self) -> Span<'static> {
    Span {
      style: self.style,
      content: self.char.to_string().into(),
    }
  }
}

pub fn split_this_is_dumb(victim: Vec<CharSpan>, pattern: char) -> Vec<Vec<CharSpan>> {
  let mut splitted = vec![];

  let mut i = 0;
  let mut last_splilt = 0;
  while i < victim.len() {
    if victim[i].char == pattern {
      splitted.push(victim[last_splilt..i].to_vec());
      last_splilt = i + 1;
    }
    i += 1;
  }

  if i <= victim.len() {
    splitted.push(victim[last_splilt..i].to_vec());
  }

  splitted
}

#[derive(Default, Clone, PartialEq)]
pub struct MyLine(pub Vec<CharSpan>);

impl From<&str> for MyLine {
  fn from(value: &str) -> Self {
    Self(value.chars().map(|x| x.into()).collect())
  }
}

impl From<String> for MyLine {
  fn from(value: String) -> Self {
    // Self(value.chars().map(|x| x.into()).collect())
    Self::from(value.as_str())
  }
}

impl From<Vec<CharSpan>> for MyLine {
  fn from(value: Vec<CharSpan>) -> Self {
    // let mut spans = Vec::with_capacity(value.len());
    //
    // for span in value {
    //   spans.push(MySpan {
    //     style: span.style,
    //     content: span.char.into(),
    //   })
    // }

    // wow look at these iters they r so cool
    Self(value)
  }
}

impl Into<Line<'_>> for MyLine {
  fn into(self) -> Line<'static> {
    Line {
      style: Style::default(),
      alignment: None,
      spans: self.0.iter().map(|x| x.into()).collect(),
    }
  }
}

impl std::fmt::Display for MyLine {
  fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for span in &self.0 {
      print!("{}", span.char)
    }

    Ok(())
  }
}

impl std::fmt::Debug for MyLine {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // println!("i gywatt called");
    for span in &self.0 {
      write!(f, "{}", span.char)?;
    }

    Ok(())
  }
}

impl MyLine {
  pub fn len(&self) -> usize {
    self.0.len()
    // let mut length = 0;
    // for span in &self.0 {
    //   length += span.len()
    // }
    // length
  }

  pub fn trim(&self) -> MyLine {
    let mut trimmed = self.clone();
    let mut i = self.0.len() as isize - 1;

    while i > 0 {
      if trimmed.0[i as usize].char == ' ' {
        trimmed.0.pop();
      } else {
        break;
      }
      i -= 1;
    }

    trimmed
  }
}

#[derive(Debug, Default, Clone)]
pub struct MultiLineString {
  pub body: String,
  pub body_ranges: Vec<RatatuiBodyRange>,
  pub usefull: Vec<CharSpan>,
  cached_lines: Vec<MyLine>,
  cached_width: u16,
  cached_length: u16,
}

fn replace_dangerous_char(input: char) -> char {
  match input {
    '\t' => {
      // Logger::log("found one");
      ' '
    }
    '\r' => ' ',
    _ => input,
  }
}

fn parse_dangerous_chars(input: String) -> String {
  input.chars().map(|c| replace_dangerous_char(c)).collect()
}

impl MultiLineString {
  pub fn new(str: &str) -> Self {
    Self {
      body: parse_dangerous_chars(str.to_string()),
      body_ranges: vec![],
      usefull: str.chars().map(|x| x.into()).collect(),
      cached_lines: vec!["".into()],
      cached_width: 0,
      cached_length: 0,
    }
  }

  pub fn new_with_ranges(str: &str, ranges: Vec<RatatuiBodyRange>) -> Self {
    let mut output = Self::new(str);

    for range in &ranges {
      let mut i = 0;
      while i < range.length as usize {
        // POTENTIAL PANIC
        output.usefull[i + range.start as usize].style = range.style;

        i += 1
      }
    }
    output.body_ranges = ranges;
    output
  }

  pub fn set_content(&mut self, string: String) {
    self.body = parse_dangerous_chars(string.clone());
    self.usefull = MyLine::from(string).0;
    self.body_ranges = vec![];
    self.cached_lines = vec![];
    self.cached_width = 0;
    self.cached_length = 0;
  }

  pub fn insert(&mut self, index: usize, char: char) {
    self
      .body
      .insert(self.body.byte_index(index), replace_dangerous_char(char));

    self.usefull.insert(
      self.body.byte_index(index),
      replace_dangerous_char(char).into(),
    );
  }

  pub fn remove(&mut self, index: usize) {
    self.body.remove(self.body.byte_index(index));
    self.usefull.remove(self.body.byte_index(index));
  }

  // I hate handling utf-8
  fn calc_lines<'s>(&self, width: u16) -> Vec<MyLine> {
    let mut lines: Vec<MyLine> = Vec::new();

    let availible_width = width as usize;

    for known_line in split_this_is_dumb(self.usefull.clone(), '\n') {
      // println!("heres the line: {}", &known_line);
      let mut new_line: Vec<CharSpan> = vec![];
      // collumn index
      let mut coldex = 0;
      // if known_line == "" {
      //   lines.push("".to_string());
      //   continue;
      // }
      // this .split() is a little sketchy but it works mostly
      for yap in split_this_is_dumb(known_line, ' ') {
        // let yap = yap.chars();

        if coldex + yap.len() <= availible_width || yap.len() == 0 {
          coldex += yap.len() + 1;
          for span in yap {
            new_line.push(span);
            // new_line.push(MySpan {
            //   style: span.style,
            //   content: span.char.into(),
            // })
          }
          new_line.push(' '.into());
        } else {
          // INCOMPLETE LOGIC!!!
          // lowkey forget what i meant by that...

          if new_line.len() > 0 {
            // lines.push(MyLine::from(new_line.clone().iter().flatten().collect()));
            lines.push(MyLine::from(new_line.clone()));
          }

          let mut index = 0;
          let mut remaining_length = yap.len();

          while remaining_length > availible_width {
            lines.push(MyLine::from(yap[index..index + availible_width].to_vec()));
            remaining_length -= availible_width;
            index += availible_width;
          }

          new_line = yap[index..].to_vec();
          coldex = new_line.len();

          if new_line.len() > 0 {
            new_line.push(' '.into());
            coldex += 1;
          }
        }
      }

      // println!("can i see this?");

      // remove the trailing ' '
      new_line.pop();
      lines.push(new_line.into());
    }

    lines
  }

  // this one isnt public cuz smthn smthn object oriented yappery
  fn update_cache<'a>(&'a mut self, width: u16) {
    self.cached_lines = self.calc_lines(width);
    self.cached_length = self.body.len() as u16;
    self.cached_width = width;
  }

  // this is the one you call
  pub fn as_lines(&mut self, width: u16) -> &Vec<MyLine> {
    // criteria for refreshing the cache
    if width != self.cached_width || self.body.len() as u16 != self.cached_length {
      self.update_cache(width);
    }

    return &self.cached_lines;
  }

  pub fn _as_owned_lines(&mut self, width: u16) -> Vec<MyLine> {
    self.as_lines(width).clone()
  }

  pub fn as_trimmed_lines(&mut self, width: u16) -> Vec<MyLine> {
    let untrimmed = self.as_lines(width);

    let mut trimmed = Vec::with_capacity(untrimmed.len());
    for line in untrimmed {
      trimmed.push(line.trim())
    }

    trimmed

    // trim_vec(untrimmed.to_vec())
    // self._as_owned_lines(width)
  }

  pub fn rows(&mut self, width: u16) -> u16 {
    self.as_lines(width).len() as u16
  }

  pub fn fit(&self, width: u16, height: u16) -> Vec<MyLine> {
    let mut fitted = self.calc_lines(width);
    let length = fitted.len();
    fitted = fitted[0..min(height as usize, length)].to_vec();
    // while fitted.len() as u16 > height {
    //   fitted.pop();
    // }

    // let shrunk = fitted[fitted.len() - 1].shrink(width);
    let last = fitted.len() - 1;
    fitted[last] = shrink_line(fitted[last].clone(), width as usize);
    fitted
  }
}

// fn trim_vec(untrimmed: Vec<Line>) -> Vec<Line> {
//   let mut trimmed: Vec<String> = vec![];
//   for line in untrimmed {
//     trimmed.push(line.trim_end().to_string());
//   }
//   trimmed
// }

fn shrink_line(line: MyLine, width: usize) -> MyLine {
  if width < line.len() {
    MyLine(line.0.as_slice()[0..width - 3].to_vec())
  } else {
    line
  }
}
