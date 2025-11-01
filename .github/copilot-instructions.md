# About

This is a rust-based "file server". That is, it's a collection of files and
folders indexed by a database and accessible via an api layer.

# Tech Stack

- rust 1.88.0 edition 2024
- sqlite3 (normal sql queries and manually creating prepared statements)
- rocket.rs
- lapin (rabbitmq wrapper)
- json for most requests / responses
- fern for logging library
- async-global-executor for async/await
- serde for (de)serialization

# Hardware / OS

designed primarily with linux in mind. Specifically a headless raspberry pi.
Currently runs flawlessly on a 3B. Windows _might_ work for some stuff, but
supporting it is not a priority

# General structure

- src/handler/* - contains all endpoints split out into general functionality
  (file_handler dealing with files, tag_handler dealing with tags, etc).
  api_handler deals with the rest api itself, such as getting the api version
  and updating auth
- src/config/* - contains files dedicated to building config from the
  FileServer.toml configuration file
- src/assets/* - contains non-rust assets used via `include_str!`; mainly sql
  files
- src/assets/migration/* - database migration files
- src/assets/queries/* - general-purpose sqlite3 files
- src/assets/init.sql - database initialization
- src/model/* - dumping ground for all models. Needs to be split out. No
  newly-created models should be put here (see the section `Organization` below)
- src/previews/* - all file preview functionality. Currently takes the first
  step outlined in the `Organization` section below
- src/queue/* - all queueing functionality. Might need to be refactored later
  but so far hasn't needed it. See the `Queue` section below
- src/repository/* - all database functionality not tied to migration or
  previews. Database models are different from the models received / sent via
  the api endpoints
- src/service/* - where most of the "business" logic lies. Needs heavy
  refactoring (see the `Organization section`)
- src/test/* - test-specific functionality. src/test/mod.rs has a lot of utility
  functions used in tests
- src/db_migrations.rs - database migration functionality. Database is versioned
  based on which upgrades have been applied. At application start, the version
  is compared with the latest upgrade version and upgrades are applied
  accordingly.

# Queue

hardware strength is limited, and generating previews takes about ~1 second on
the target machine. Preview generation is delayed until no endpoints have been
hit for a user-defined amount of time (defaults to 30 seconds).
`file_preview_consumer` is called in src/main.rs to set this up. All queue
functionality is disabled during tests

# Organization

the project was originally organized after general java+spring organization -
controller/handler layer, repository layer, service layer, etc. However that has
proven to not _really_ work with how I organize stuff. Going forward, I'd like
to use cargo workspaces to split out everything into its own crate (e.g. the
file handler, service layer, repository layer, and models all go in a specific
crate, same for folders, etc). Until then though, everything is in the same
crate and will be slowly migrated to models and _then_ crates to make the
migration easier

# Testing

general test structure takes this format (pulled from code snippets in
.vscode/snippets.code-snippets):

```rs
#[test]
fn test_name() {
    // test contents
}
```

most tests won't use this though, because they will need to perform database
operations or file system operations. each test gets its own database and folder
its files go into. The general test structure for those tests should be

```rs
#[test]
fn test_name() {
    init_db_folder();
    // test contents
    cleanup();
}
```

when just _stubbing_ tests, use `crate::fail();` in place of the `test contents`
comment.

most utility functions are in `src/test/mod.rs`. Use functions from there when

- performing filesystem operations
- updating the database
- getting the current date
- updating file/folder tags
- getting authorization

these functions should _**ONLY**_ be used during tests and _**ONLY**_ when the
primary function of the test doesn't require them. e.g. when writing a test that
create_file successfully saves a file to the disk and database, don't use
`create_file_db_entry` or `create_file_disk`. However (and still an example) if
the test is about ensuring a preview is created successfully, it is ok to use
those functions to set up the scenario for the test

# Code Style

prefer modern rust features, but don't overengineer. Keep things simple, and
most things you won't need to introduce.

`format!` (and related) strings should always prefer to keep the variables in
the string itself rather than separate arguments, if possible. For a contrived
example _never_ do this:

```rs
let x = 1;
format!("x: {}", x);
```

instead do this:

```rs
let x = 1;
format!("x: {x}");
```
