use std::cmp::min;

use crate::MyStringUtils;

#[derive(Debug, Default, Clone)]
pub struct MultiLineString {
  pub body: String,
  cached_lines: Vec<String>,
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

impl MultiLineString {
  pub fn new(str: &str) -> Self {
    Self {
      body: str.to_string(),
      cached_lines: vec!["".to_string()],
      cached_width: 0,
      cached_length: 0,
    }
  }

  // I hate handling utf-8
  fn calc_lines(&self, width: u16) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut new_line = String::from("");

    // collumn index
    let mut coldex = 0;
    // let availible_width = (term_width as f32 * settings.message_width_ratio + 0.5) as usize;
    let availible_width = width as usize;

    // this .split() is a little sketchy but it works mostly
    for yap in self.body.split(" ") {
      let yap = yap.chars();
      let mut length = yap.clone().count();

      if coldex + length <= availible_width || length == 0 {
        new_line.push_str(yap.as_str());
        new_line.push_str(" ");
        coldex += length + 1;
      } else {
        // INCOMPLETE LOGIC!!!
        if new_line != "" {
          lines.push(new_line.clone());
        }

        let mut index = 0;

        let yap: Vec<_> = yap.collect();
        while length >= availible_width {
          lines.push(string_from_chars(&yap[index..index + availible_width]));
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

    // remove the trailing ' '
    new_line.pop();
    lines.push(new_line);
    lines
  }

  // this one isnt public cuz smthn smthn object oriented yappery
  fn update_cache(&mut self, width: u16) {
    self.cached_lines = self.calc_lines(width);
    self.cached_length = self.body.len() as u16;
    self.cached_width = width;
  }

  // this is the one you call
  pub fn as_lines(&mut self, width: u16) -> &Vec<String> {
    // criteria for refreshing the cache
    if width != self.cached_width || self.body.len() as u16 != self.cached_length {
      self.update_cache(width);
    }

    return &self.cached_lines;
  }

  pub fn _as_owned_lines(&mut self, width: u16) -> Vec<String> {
    self.as_lines(width).clone()
  }

  pub fn as_trimmed_lines(&mut self, width: u16) -> Vec<String> {
    let untrimmed = self.as_lines(width);
    trim_vec(untrimmed.to_vec())
  }

  pub fn rows(&mut self, width: u16) -> u16 {
    self.as_lines(width).len() as u16
  }

  pub fn fit(&self, width: u16, height: u16) -> Vec<String> {
    let mut fitted = trim_vec(self.calc_lines(width));
    let length = fitted.len();
    fitted = fitted[0..min(height as usize, length)].to_vec();
    // while fitted.len() as u16 > height {
    //   fitted.pop();
    // }

    // let shrunk = fitted[fitted.len() - 1].shrink(width);
    let last = fitted.len() - 1;
    fitted[last] = fitted[last].shrink(width);
    fitted
  }
}

fn trim_vec(untrimmed: Vec<String>) -> Vec<String> {
  let mut trimmed: Vec<String> = vec![];
  for line in untrimmed {
    trimmed.push(line.trim_end().to_string());
  }
  trimmed
}
