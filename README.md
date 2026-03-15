# Signal-TUI

<div align="center">

[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/) ![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white) ![License](https://img.shields.io/crates/l/ratatui?style=flat-square&color=1370D3)

</div>
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

### NOTE: If the chats fail to load on first startup, send yourself a message via "note to self" and reopen the application

* neovim-esque modal interface
* i / esc to toggle between "normal" (move around) and "insert" (type messages)
* j / k / h / l or arrow keys to navigate in normal mode
* o to enter open options on selected message (reply, edit, etc...)
* x to cancel replying to a message
* q to quit
* ? to open help menu

## Known Issues
* Chats only become known after receiving a message from them
* Editing a message will sometimes send a new message with the new content, instead of changing the original message
