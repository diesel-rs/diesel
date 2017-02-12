# Contributing

Thanks for wanting to contribute to Diesel! We are very much looking forward to your suggestions, bug reports, and pull requests.

## Setting up diesel locally

1. Install Rust using [rustup], which allows you to easily switch between Rust versions.
2. Install the system libraries needed to interface with the database systems you which to use.

    These are the same as when compiling diesel. In general, it is a good idea to have _all_ drivers installed so you can run all tests locally.

    *Shortcut:* On macOS, you don't need to install anything to work with Postgres and SQLite.
3. Clone this repository and open it in your favorite editor.
4. Create a `.env` file in the `diesel/` directory, and add the connection details for your databases.

    For example:

    ```
    PG_DATABASE_URL=postgresql://pascal@localhost/diesel_test
    MYSQL_DATABASE_URL=mysql://root@localhost/diesel_test
    SQLITE_DATABASE_URL=/tmp/diesel_test.sqlite
    ```
5. Now, try running the test suite to confirm everything works for you locally by executing `bin/test`. (Initially, this will take a while to compile everything.)

[rustup]: https://www.rustup.rs
