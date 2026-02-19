use std::vec;

use crate::Message;
use crate::MultiLineString;
use crate::model::CharSpan;
use crate::model::MyLine;
use crate::model::split_this_is_dumb;

// mod multi_line_string;

#[test]
fn test_tests() {
  assert!(true);
}

fn vecs_equal<T: PartialEq>(vec1: Vec<T>, vec2: Vec<T>) -> bool {
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
fn test_dumb_split() {
  let line = MyLine::from("one word");
  let output = split_this_is_dumb(line.0, '\n');
  // println!("{:?}", output);
  assert_eq!(output, vec![MyLine::from("one word").0])
}

#[test]
fn test_trimmed_lines() {
  struct Test<'a> {
    string: &'a str,
    expected: Vec<&'a str>,
    width: u16,
  }

  let tests = vec![
    Test {
      string: "this is myy message",
      expected: vec!["this", "is", "myy", "messa", "ge"],
      width: 5,
    },
    Test {
      string: "wow      big space",
      expected: vec!["wow", "big", "space"],
      width: 5,
    },
  ];

  for test in tests {
    let mut body = MultiLineString::new(test.string);
    let output = body.as_trimmed_lines(test.width);

    let mut expected: Vec<MyLine> = vec![];
    for line in test.expected {
      expected.push(line.chars().map(|x| x.into()).collect::<Vec<CharSpan>>().into())
    }
    assert_eq!(&output, &expected);
  }
  //
  // body = MultiLineString::new("we       have space and");
  //
  // let output = body.as_trimmed_lines(width);
  //
  // // for line in &output {
  // //   println!("{}|end", line);
  // // }
  //
  // let mut expected: Vec<String> = Vec::new();
  // for line in vec!["we", "have", "space", "and"] {
  //   expected.push(line.to_string());
  // }
  // assert!(vecs_equal(output.to_vec(), expected));

  // body = MultiLineString::new("");
  //
  // assert!(vecs_equal(body.as_lines(width).to_vec(), vec!["".to_string()]));
  //

  // body = MultiLineString::new("first_line\nsecond_line");
  //
  // let mut expected: Vec<String> = Vec::new();
  // for line in vec!["first_line", "second_line"] {
  //   expected.push(line.to_string());
  // }
  //
  // width = 11;
  // let output = body.as_trimmed_lines(width);
  // for line in &output {
  //   println!("{}|end", line);
  // }
  //
  // assert!(vecs_equal(output.to_vec(), expected));
  // width = 20;
  //
  // let help_text_lines = vec![
  //   "Interact with the meshtastci-2-signal gateway bot",
  //   "",
  //   "Commands:",
  //   "\t/channel\t\tDisplay information about the meshtastic channel",
  //   "\t/help\t\tDisplay this help message",
  // ];
  //
  // let mut help_text: String = Default::default();
  // for line in help_text_lines {
  //   help_text.push_str(line);
  //   help_text.push('\n');
  // }
  //
  // body.set_content(help_text);
  // let output = body.as_trimmed_lines(width);
  //
  // for line in &output {
  //   println!("{}|end", line);
  // }
  //
  // assert!(false);
}

// #[test]
// fn i_wanna_see() {
//   let mut message = Message::default();
//   message.body = MultiLineString::init(
//     "first message lets make this message super looong jjafkldjaflk it was not long enough last time time to yap fr",
//   );
//   let width = 68;
//
//   let output = message.body.as_lines(width);
//
//   for line in output {
//     println!("{}", line);
//   }
//
//   // assert!(false);
//   assert!(true);
// }

// #[test]
// fn im_so_tired() {
//   let two_hours = TimeDelta::hours(2);
//   let mut two_hours_ago = Utc::now();
//   two_hours_ago = two_hours_ago.checked_sub_signed(two_hours).unwrap();
//
//   let formatted = format_duration(&two_hours_ago);
//
//   println!("{}", formatted);
//
//   assert_eq!(formatted, "2h");
//
//   let now = Utc::now();
//
//   let formatted = format_duration(&now);
//
//   println!("{}", formatted);
//
//   assert_eq!(formatted, "Now")
// }
