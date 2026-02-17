use std::cmp::min;

use crate::{MyStringUtils, RatatuiBodyRange, logger::Logger};
use futures::stream::Iter;
use ratatui::{
  style::Style,
  text::{Line, Span},
};

#[derive(Debug, Default, Clone)]
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

impl MySpan {
  fn len(&self) -> usize {
    self.content.len()
  }
}

#[derive(Debug, Default, Clone)]
pub struct MyLine(Vec<MySpan>);

impl From<&str> for MyLine {
  fn from(value: &str) -> Self {
    Self(vec![value.into()])
  }
}

impl From<String> for MyLine {
  fn from(value: String) -> Self {
    Self(vec![value.into()])
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

impl MyLine {
  pub fn len(&self) -> usize {
    let mut length = 0;
    for span in &self.0 {
      length += span.len()
    }
    length
  }
}

#[derive(Debug, Default, Clone)]
pub struct MultiLineString {
  pub body: String,
  pub body_ranges: Vec<RatatuiBodyRange>,
  cached_lines: Vec<MyLine>,
  cached_width: u16,
  cached_length: u16,
}

fn string_from_chars(chars: &[char]) -> String {
  let mut string = String::new();
  for chr in chars {
    string.push_str(&chr.to_string());
  }

  string
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
      cached_lines: vec!["".into()],
      cached_width: 0,
      cached_length: 0,
    }
  }

  pub fn new_with_ranges(str: &str, ranges: Vec<RatatuiBodyRange>) -> Self {
    let mut output = Self::new(str);
    output.body_ranges = ranges;
    output
  }

  pub fn set_content(&mut self, string: String) {
    self.body = parse_dangerous_chars(string);
    self.cached_lines = vec![];
    self.cached_width = 0;
    self.cached_length = 0;
  }

  pub fn insert(&mut self, index: usize, char: char) {
    self
      .body
      .insert(self.body.byte_index(index), replace_dangerous_char(char));
  }

  pub fn remove(&mut self, index: usize) {
    self.body.remove(self.body.byte_index(index));
  }

  // I hate handling utf-8
  fn calc_lines<'s>(&self, width: u16) -> Vec<MyLine> {
    let mut lines: Vec<MyLine> = Vec::new();

    let availible_width = width as usize;

    for known_line in self.body.split("\n") {
      // println!("heres the line: {}", &known_line);
      let mut new_line = String::from("");
      // collumn index
      let mut coldex = 0;
      // if known_line == "" {
      //   lines.push("".to_string());
      //   continue;
      // }
      // this .split() is a little sketchy but it works mostly
      for yap in known_line.split(" ") {
        let yap = yap.chars();
        let mut length = yap.clone().count();

        if coldex + length <= availible_width || length == 0 {
          new_line.push_str(yap.as_str());
          new_line.push_str(" ");
          coldex += length + 1;
        } else {
          // println!("shouldnt go here");
          // INCOMPLETE LOGIC!!!
          if new_line != "" {
            lines.push(new_line.clone().into());
          }

          let mut index = 0;

          let yap: Vec<_> = yap.collect();
          while length >= availible_width {
            lines.push(string_from_chars(&yap[index..index + availible_width]).into());
            length -= availible_width;
            index += availible_width;
          }

          new_line = string_from_chars(&yap[index..]);
          coldex = new_line.len();

          if new_line.len() > 0 {
            new_line.push_str(" ");
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
    // let untrimmed = self.as_lines(width);
    // trim_vec(untrimmed.to_vec())
    self._as_owned_lines(width)
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
