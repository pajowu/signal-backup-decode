# signal backup decoder

This repository contains a small programm to decode a backup produced by [Signal Android](https://github.com/signalapp/Signal-Android).

## Installation

**Rust v1.40 or higher is required**

```
cargo install signal-backup-decode
```

## Feature Flags

This tool depends on parsed protoc files. A pre-generated version is included with in this repo, they can be regenerated using the feature flag `rebuild-protobuf`.

**For regenerating the protobuf-files this tool, `protoc` has to be installed.**

- Debian: ```apt install protobuf-compiler```
- Arch: ```pacman -S protobuf```

Once `protoc` is installed, this tool can be installed using `cargo`:

```
cargo install --features "rebuild-protobuf" signal-backup-decode
```


## Usage

This tool is run as `signal-backup-decode`. See `signal-backup-decode --help`:

```
signal-backup-decode [FLAGS] [OPTIONS] <INPUT> --sqlite-path <sqlite_path> <--password <PASSWORD>|--password_file <FILE>>
pajowu <pajowu@pajowu.de>
A simple tool to decode signal backups

USAGE:
    signal-backup-decode [FLAGS] [OPTIONS] <INPUT> --sqlite-path <sqlite_path> <--password <PASSWORD>|--password_file <FILE>>

FLAGS:
    -h, --help             Prints help information
        --no-tmp-sqlite    Do not use a temporary file for the sqlite database
        --no-verify-mac    Do not verify the HMAC of each frame in the backup
    -V, --version          Prints version information

OPTIONS:
        --attachment-path <attachment_path>    Directory to save attachments to [default: attachments]
        --avatar-path <avatar_path>            Directory to save avatar images to [default: avatars]
        --config-path <config_path>            Directory to save config files to [default: config]
    -o, --output-path <FOLDER>                 Directory to save output to
    -f, --password_file <FILE>                 File to read the Backup password from
    -p, --password <PASSWORD>                  Backup password (30 digits, with or without spaces)
        --sqlite-path <sqlite_path>            File to store the sqlite database in [default:
                                               output_path/signal_backup.db]

ARGS:
    <INPUT>    Sets the input file to use

```

## License

This repository is under the GPLv3 License.

The proto/Backups.proto file is taken and derived from the [Signal Android Source Code](https://github.com/signalapp/Signal-Android) with the following copyright notice:

```
/**
 * Copyright (C) 2018 Open Whisper Systems
 *
 * Licensed according to the LICENSE file in this repository.
 */
```
