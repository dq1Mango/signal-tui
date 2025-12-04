use chrono::Local;
// use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;

pub struct Logger {
  // file: File,
}

static FILE_NAME: &str = "log.txt";

impl Logger {
  pub fn init(file_name: &str) -> Self {
    let mut file = OpenOptions::new()
      .write(true)
      .truncate(true)
      .create(true)
      .open(file_name)
      .expect("am i goated?");

    let result = writeln!(file, "=== START OF LOG ===\n");

    result.expect("am i goated?");

    // file = OpenOptions::new()
    //   .append(true)
    //   .create(true)
    //   .open(file_name)
    //   .expect("kaboom");

    let logger = Logger {};

    logger
  }

  // pub fn log(&mut self, log: String) {
  //   let date_time = Local::now();
  //   let formatted = format!("{}", date_time.format("%d/%m/%Y %H:%M"));
  //   // no clue why this works
  //   writeln!(self.file, "{}", log).expect("kaboom");
  // }
  //
  pub fn _clear() {
    let mut file = OpenOptions::new()
      .write(true)
      .truncate(true)
      .create(true)
      .open(FILE_NAME)
      .expect("am i goated?");

    writeln!(file, "=== START OF LOG === ").expect("kaboom");
  }

  pub fn log(log: String) {
    let mut file = OpenOptions::new()
      .append(true)
      .create(true)
      .open(FILE_NAME)
      .expect("kaboom");
    let date_time = Local::now();
    let formatted = format!("{}", date_time.format("%H:%M:%S"));
    writeln!(file, "{}: {}", formatted, log).expect("kaboom");
  }
}
