use std::{
  fs::{self, File},
  io::Write,
};

use serde::{Deserialize, Serialize};

use crate::{config_dir_path, logger::Logger};

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
  pub borders: bool,
  pub message_width_ratio: f32,
  pub md_formatting: bool,
  // pub _identity: String,
}

pub fn config_file_path() -> String {
  config_dir_path().join("config.toml").display().to_string()
}

impl Settings {
  pub fn defaults() -> Self {
    Self {
      borders: true,
      message_width_ratio: 0.8,
      md_formatting: true,
    }
  }
  pub fn init() -> Self {
    // let mut file = match File::open("config.toml") {
    //   Ok(f) => f,
    //   Err(err) => {
    //     Logger::log("unable to open file 'config.toml'");
    //     Logger::log(format!("heres an error also {}", err));
    //     panic!();
    //   }
    // };
    //
    // let mut contents = String::new();
    // file.read_to_string(&mut contents).expect("cmon no way this fails");

    // let test = std::fs::File::read_to_string(&mut self, buf)
    //
    match fs::read_to_string(config_file_path()) {
      Ok(c) => match toml::from_str(&c) {
        Ok(config) => config,
        Err(err) => {
          Logger::log(format!("unable to parse config file: {}", err));
          Self::defaults()
        }
      },
      Err(err) => {
        Logger::log("unable to open file 'config.toml'");
        Logger::log(format!("heres an error also {}", err));

        Logger::log("writing defaults to config file ...");
        let mut file =
          File::create_new(config_file_path()).expect("hmmm it wasnt there a second ago???");
        let err = File::write_all(
          &mut file,
          toml::to_string(&Self::defaults()).unwrap().as_bytes(),
        );
        if let Err(err) = err {
          Logger::log(format!("wow we cant do anything right: {}", err));
        }

        Self::defaults()
      }
    }
  }
}
