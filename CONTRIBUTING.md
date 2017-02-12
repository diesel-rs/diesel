# Contributing

Thanks for wanting to contribute to Diesel! We are very much looking forward to your suggestions, bug reports, and pull requests.

## Setting up diesel locally

1. Install Rust using [rustup], which allows you to easily switch between Rust versions.
2. Install the system libraries needed to interface with the database systems you which to use.

    These are the same as when compiling diesel. In general, it is a good idea to have _all_ drivers installed so you can run all tests locally.

    *Shortcut:* On macOS, you don't need to install anything to work with SQLite and for PostgreSQL you'll only the server (`libpq` is installed by default). So, to get started, `brew install postgresql mysql` and follow the instructions shown to set up the database servers.
3. Clone this repository and open it in your favorite editor.
4. Create a `.env` file in the `diesel/` directory, and add the connection details for your databases.

    For example:

    ```
    PG_DATABASE_URL=postgresql://localhost/diesel_test
    SQLITE_DATABASE_URL=/tmp/diesel_test.sqlite
    MYSQL_DATABASE_URL=mysql://localhost/diesel_test
    MYSQL_UNIT_TEST_DATABASE_URL=mysql://localhost/diesel_unit_tests
    ```

    *Note:* If you didn't specify the MySQL user to be one with elevated permissions, you'll want to a command like ```mysql -c "GRANT ALL ON `diesel_%`.* TO ''@'localhost';" -uroot```, or something similar for the user that you've specified.
5. Now, try running the test suite to confirm everything works for you locally by executing `bin/test`. (Initially, this will take a while to compile everything.)

[rustup]: https://www.rustup.rs
