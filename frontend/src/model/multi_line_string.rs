use crate::{MyStringUtils, RatatuiBodyRange, logger::Logger};
use presage::proto::BodyRange;
use ratatui::{
  style::{Style, Stylize},
  text::{Line, Span},
};
use std::{cmp::min, mem::take};

// will eventually need to make the "grapheme cluster"-span
// TODO more like TODO-NT
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

pub fn style_to_pattern(style: fn(Style) -> Style) -> Vec<char> {
  // surely this ends up in .DATA right?
  let formats: Vec<(Vec<char>, fn(Style) -> Style)> = vec![(vec!['*'], Style::italic), (vec!['*', '*'], Style::bold)];

  for (pattern, func) in formats {
    if func == style {
      return pattern;
    }
  }

  return vec![];
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
        let span = &mut output.usefull[i + range.start as usize];
        span.style = (range.style)(span.style);

        i += 1
      }
    }
    output.body_ranges = ranges;
    output
  }
  pub fn is_pattern(&self, index: usize, pattern: &Vec<char>) -> bool {
    // nice oob check
    if index + pattern.len() > self.usefull.len() {
      return false;
    }

    for (j, chr) in pattern.iter().enumerate() {
      if self.usefull[index + j].char != *chr {
        return false;
      }
    }

    // !self.is_pattern(index + pattern.len(), pattern)
    true
  }
  pub fn is_isolated_pattern(&self, index: usize, pattern: &Vec<char>) -> bool {
    let check_next = self.is_pattern(index, pattern) && !self.is_pattern(index + pattern.len(), pattern);
    if pattern.len() > index {
      check_next
    } else {
      check_next && !self.is_pattern(index - pattern.len(), pattern)
    }
  }
  pub fn find_dyck_pattern(&self, pattern: &Vec<char>, start: usize, taken: &mut Vec<usize>) -> Option<(usize, usize)> {
    let mut i = start;

    while i < self.usefull.len() {
      if self.is_pattern(i, pattern) && !taken.contains(&i) {
        // Logger::log("found first part of dyck");
        i += pattern.len();

        if self.is_pattern(i, pattern) {
          // Logger::log("aint it chief");
          // Logger::log(format!("{:?}", pattern));
          // Logger::log(format!("{}", i));

          i += pattern.len();
          continue;
        }

        let start = i;

        while i < self.usefull.len() {
          if self.is_pattern(i, pattern) && !taken.contains(&i) {
            //&& !self.is_pattern(i + pattern.len(), pattern) {
            return Some((start, i));
          }
          i += 1;
        }

        return None;
      }

      i += 1;
    }

    None
  }
  pub fn md_formatting_ranges(&self) -> Vec<RatatuiBodyRange> {
    let mut ranges = vec![];
    let mut taken_indicies = vec![];

    let formats: Vec<(Vec<char>, fn(Style) -> Style)> = vec![(vec!['*', '*'], Style::bold), (vec!['*'], Style::italic)];

    for (pattern, style) in formats {
      Logger::log(format!("{:?}", taken_indicies));
      Logger::log("");
      let mut i = 0;
      while let Some(range) = self.find_dyck_pattern(&pattern, i, &mut taken_indicies) {
        // Logger::log("found some fun formatting ranges");
        i = range.1 + pattern.len();
        let length = (range.1 - range.0) as u32;

        for i in 0..pattern.len() as usize {
          taken_indicies.push(range.0 - 1 - i);
          taken_indicies.push(range.1 + i);
        }

        ranges.push(RatatuiBodyRange {
          start: range.0 as u32,
          length: length,
          style: style,
        });
      }
    }

    ranges
  }

  pub fn apply_body_ranges(&mut self) {
    for span in &mut self.usefull {
      span.style = Style::default();
    }

    // let make_bold = Style::bold;

    for range in &self.body_ranges {
      for i in range.start..range.start + range.length {
        self.usefull[i as usize].style = (range.style)(self.usefull[i as usize].style);
      }
    }
  }

  pub fn update_md_formatting(&mut self) {
    self.body_ranges = self.md_formatting_ranges();
    self.apply_body_ranges();
  }

  /// Does this add a doc comment,
  /// also only call this one if u r done with the MultiLineString,
  /// (it kind of nukes it)
  pub fn extract_formatted(&mut self) {
    // shoooouldnt need to update again
    // self.update_md_formatting();
    //
    let len_ranges = self.body_ranges.len();
    let mut i = 0;
    while i < len_ranges {
      let range = &mut self.body_ranges[i];
      let patter_len = style_to_pattern(range.style).len() as u32;
      Logger::log(format!("in this range: {:?}", &range));
      Logger::log(format!("patter this long: {}", patter_len));

      for _ in 0..patter_len {
        self.usefull.remove((range.start + range.length) as usize);
      }
      range.start -= patter_len;

      for _ in 0..patter_len {
        self.usefull.remove(range.start as usize);
      }

      let start = range.start;
      let length = range.length;

      let mut j = 0;
      while j < len_ranges {
        if i == j {
          j += 1;
          continue;
        }

        let other_range = &mut self.body_ranges[j];

        if other_range.start > start {
          other_range.start -= patter_len;
        } else if other_range.start + other_range.length > start {
          other_range.length -= patter_len;
        }

        if other_range.start > start + length {
          other_range.start -= patter_len;
        } else if other_range.start + other_range.length > start + length {
          other_range.length -= patter_len;
        }

        j += 1;
      }

      i += 1;
    }
  }

  pub fn set_content(&mut self, string: String) {
    self.body = parse_dangerous_chars(string.clone());
    self.usefull = MyLine::from(string).0;
    self.body_ranges = vec![];
    self.cached_lines = vec![];
    self.cached_width = 0;
    self.cached_length = 0;

    self.update_md_formatting();
  }

  pub fn insert(&mut self, index: usize, char: char) {
    let safe_char = replace_dangerous_char(char);
    self.body.insert(self.body.byte_index(index), safe_char);

    self.usefull.insert(index, safe_char.into());
    self.update_md_formatting();
  }

  pub fn remove(&mut self, index: usize) {
    self.body.remove(self.body.byte_index(index));
    self.usefull.remove(index);
    self.update_md_formatting();
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

  pub fn as_string(&self) -> String {
    self.usefull.clone().iter().map(|x| x.char).collect()
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
