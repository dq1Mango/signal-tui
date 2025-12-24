# Signal-TUI

An unnofical minimal terminal interface for the Signal messaging service

### insert demo here ...

## Installation

### Building From Source

First install [Rust](https://rust-lang.org/tools/install/)

(on nixos you can simply enter the provided shell after cloning the repo)

Clone this repo

    git clone git@github.com:dq1Mango/signal-tui.git

Enter the new directory

    cd signal-tui

Build the binary (should take ~1 minute)
    
    cargo build --release

The binary will be ./target/release/signal-tui

## Usage

* on first startup scan the qr code to link device to your signal account 
(maybe one day u can register an account but that day is not today)

* neovim-esque modal interface
* i / esc to toggle between "normal" (move around) and "insert" (type messages)
* j / k / h / l or arrow keys to navigate in normal mode
* o to enter open options on selected message (reply, edit, etc...)

[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/)
    
