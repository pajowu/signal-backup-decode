# signal backup decoder

This repository contains a small programm to decode a backup produced by [Signal Android](https://github.com/signalapp/Signal-Android).


## Installation

**Rust v1.41 or higher is required**

```
cargo install signal-backup-decode
```

On Debian/Ubuntu you will need the following packages:

```
sudo apt install libsqlite3-dev libssl-dev pkg-config
```


## Usage

This tool is run as `signal-backup-decode`. See `signal-backup-decode --help`:

```
signal-backup-decode 0.2.1
pajowu <pajowu@pajowu.de>
A simple tool to decode signal backups

USAGE:
    signal-backup-decode [FLAGS] [OPTIONS] <INPUT> <--password <PASSWORD>|--password-file <FILE>|--password-command <COMMAND>>

FLAGS:
    -f, --force              Overwrite existing output files
    -h, --help               Prints help information
        --no-in-memory-db    Do not use in memory sqlite database. Database is immediately created on disk (only
                             considered with output type RAW).
        --no-verify-mac      Do not verify the HMAC of each frame in the backup
    -V, --version            Prints version information

OPTIONS:
    -v, --verbosity <LEVEL>             Verbosity level, either DEBUG, INFO, WARN, or ERROR
    -o, --output-path <FOLDER>          Directory to save output to. If not given, input file directory is used
    -t, --output-type <TYPE>            Output type, either RAW, CSV or NONE
        --password-command <COMMAND>    Read backup password from stdout from COMMAND
        --password-file <FILE>          File to read the backup password from
    -p, --password <PASSWORD>           Backup password (30 digits, with or without spaces)

ARGS:
    <INPUT>    Sets the input file to use
```

If you want to overwrite an existing backup, use the `-f` flag. Output type 
`NONE` can be useful to check the backup file for corrupted frames but no 
output is written to disk. Only the first line is read from 
`--password-command` and `--password-file`.


## Feature Flags

This tool depends on parsed protoc files. A pre-generated version is included with in this repo, they can be regenerated using the feature flag `rebuild-protobuf`.

**For regenerating the protobuf-files this tool, `protoc` has to be installed.**

- Debian: ```apt install protobuf-compiler```
- Arch: ```pacman -S protobuf```

Once `protoc` is installed, this tool can be installed using `cargo`:

```
cargo install --features "rebuild-protobuf" signal-backup-decode
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
