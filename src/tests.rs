use std::ops::Sub;

use chrono::Duration;
use chrono::TimeDelta;
use chrono::Utc;

use crate::Message;
use crate::MultiLineString;
use crate::format_duration;

// mod multi_line_string;

#[test]
fn test_tests() {
  assert!(true);
}

fn vecs_equal(vec1: Vec<String>, vec2: Vec<String>) -> bool {
  if vec1.len() != vec2.len() {
    return false;
  }

  let mut i = 0;
  while i < vec1.len() {
    if vec1[i] != vec2[i] {
      return false;
    }
    i += 1;
  }

  true
}

#[test]
fn test_split_into_lines() {
  let width = 5;

  let mut message = Message::default();
  message.body = MultiLineString::init("this is myy message");

  let output = message.body.as_lines(width);

  for line in output {
    println!("{}|end", line);
  }

  let mut expected: Vec<String> = Vec::new();
  for line in vec!["this ", "is ", "myy ", "messa", "ge"] {
    expected.push(line.to_string());
  }

  assert!(vecs_equal(output.to_vec(), expected));

  message.body = MultiLineString::init("we       have space and");

  let output = message.body.as_trimmed_lines(width);

  for line in &output {
    println!("{}|end", line);
  }

  let mut expected: Vec<String> = Vec::new();
  for line in vec!["we", "have", "space", "and"] {
    expected.push(line.to_string());
  }

  assert!(vecs_equal(output.to_vec(), expected))
}

#[test]
fn i_wanna_see() {
  let mut message = Message::default();
  message.body = MultiLineString::init(
    "first message lets make this message super looong jjafkldjaflk it was not long enough last time time to yap fr",
  );
  let width = 68;

  let output = message.body.as_lines(width);

  for line in output {
    println!("{}", line);
  }

  // assert!(false);
  assert!(true);
}

#[test]
fn im_so_tired() {
  let two_hours = TimeDelta::hours(2);
  let mut two_hours_ago = Utc::now();
  two_hours_ago = two_hours_ago.checked_sub_signed(two_hours).unwrap();

  let formatted = format_duration(&two_hours_ago);

  println!("{}", formatted);

  assert_eq!(formatted, "2h");

  let now = Utc::now();

  let formatted = format_duration(&now);

  println!("{}", formatted);

  assert_eq!(formatted, "Now")
}
