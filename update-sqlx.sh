#!/bin/sh

# small script to install sqlx-cli tool and "prepare" the db (whatever that means)
#
# also if u r on windows try wsl and good luck :)

cargo install sqlx-cli --no-default-features --features sqlite,openssl-vendored
cargo sqlx prepare --workspace --database-url "sqlite://database/test.db"
