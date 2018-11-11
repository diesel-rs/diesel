# Contributing

Thanks for your interest in contributing to Diesel! We very much look forward to
your suggestions, bug reports, and pull requests.

We run an active [Gitter
channel](https://gitter.im/diesel-rs/diesel) where you can ask Diesel-related questions and
get help. Feel free to ask there before opening a GitHub issue or
pull request.

*Note:* Anyone who interacts with Diesel in any space, including but not
limited to this GitHub repository, must follow our [code of
conduct](https://github.com/diesel-rs/diesel/blob/master/code_of_conduct.md).


## Submitting bug reports

Have a look at our [issue tracker]. If you can't find an issue (open or closed)
describing your problem (or a very similar one) there, please open a new issue with
the following details:

- Which versions of Rust and Diesel are you using?
- Which feature flags are you using?
- What are you trying to accomplish?
- What is the full error you are seeing?
- How can we reproduce this?
  - Please quote as much of your code as needed to reproduce (best link to a
    public repository or [Gist])
  - Please post as much of your database schema as is relevant to your error

[issue tracker]: https://github.com/diesel-rs/diesel/issues
[Gist]: https://gist.github.com

Thank you! We'll try to respond as quickly as possible.


## Submitting feature requests

If you can't find an issue (open or closed) describing your idea on our [issue
tracker], open an issue. Adding answers to the following
questions in your description is +1:

- What do you want to do, and how do you expect Diesel to support you with that?
- How might this be added to Diesel?
- What are possible alternatives?
- Are there any disadvantages?

Thank you! We'll try to respond as quickly as possible.


## Contribute code to Diesel

### Setting up Diesel locally

1. Install Rust using [rustup], which allows you to easily switch between Rust
   versions. Diesel currently supports Rust Stable, Nightly, Rust Beta.

2. Install the system libraries needed to interface with the database systems
   you wish to use.

   These are the same as when compiling Diesel. It's generally a good idea
   to install _all_ drivers so you can run all tests locally.

   *Shortcut:* On macOS, you don't need to install anything to work with SQLite.
   For PostgreSQL, you'll only need the server (`libpq` is installed by
   default). To get started, `brew install postgresql mysql` and follow the
   instructions shown to set up the database servers.
3. Clone this repository and open it in your favorite editor.
4. Create a `.env` file in this directory, and add the connection details for
   your databases.

   *Additional note:* The MySQL tests currently fail when running on MySQL 5.6
   or lower. If you have 5.6 or lower installed locally and cannot upgrade for
   some reason, you may want to consider setting up Docker as mentioned below.

   See [.env.sample](.env.sample) for an example that works with a trivial
   local setup.

   *Note:* If you didn't specify the MySQL user to be one with elevated
   permissions, you'll want to run a command like ```mysql -c "GRANT ALL ON
   `diesel_%`.* TO ''@'localhost';" -uroot```, or something similar for the
   user that you've specified.

   If you have [Docker](https://docker.io), the following snippet might help you
   to get Postgres and MySQL running (with the above `.env` file):

   ```bash
   #!/usr/bin/env sh
   set -e
   docker run -d --name diesel.mysql -p 3306:3306 -e MYSQL_ALLOW_EMPTY_PASSWORD=true mysql
   while
     sleep 1;
     ! echo 'CREATE DATABASE diesel_test; CREATE DATABASE diesel_unit_test;' | docker exec -i diesel.mysql mysql
   do sleep 1; done

   docker run -d --name diesel.postgres -p 5432:5432 postgres
   while
     sleep 1;
     ! echo 'CREATE DATABASE diesel_test;' | docker exec -i diesel.postgres psql -U postgres
   do :; done
   ```

   If you want to use docker-compose, you can execute docker-compose command like this.

    ```bash
    $ docker-compose up
    ```

5. Now, try running the test suite to confirm everything works for you locally
   by executing `bin/test`. (Initially, this will take a while to compile
   everything.)

[rustup]: https://www.rustup.rs

### Coding Style

We follow the [Rust Style Guide](https://github.com/rust-lang-nursery/fmt-rfcs/blob/master/guide/guide.md), enforced using [rustfmt](https://github.com/rust-lang-nursery/rustfmt).
To run rustfmt tests locally:

1. Use rustup to set rust toolchain to the version specified in the
   [rust-toolchain file](./rust-toolchain).

2. Install the rustfmt and clippy by running
   ```
   rustup component add rustfmt-preview
   rustup component add clippy-preview
   ```

3. Run clippy using cargo from the root of your diesel repo.
   ```
   cargo clippy
   ```
   Each PR needs to compile without warning.

4. Run rustfmt using cargo from the root of your diesel repo.

   To see changes that need to be made, run

   ```
   cargo fmt --all -- --check
   ```

   If all code is properly formatted (e.g. if you have not made any changes),
   this should run without error or output.
   If your code needs to be reformatted,
   you will see a diff between your code and properly formatted code.
   If you see code here that you didn't make any changes to
   then you are probably running the wrong version of rustfmt.
   Once you are ready to apply the formatting changes, run

   ```
   cargo fmt --all
   ```

   You won't see any output, but all your files will be corrected.

You can also use rustfmt to make corrections or highlight issues in your editor.
Check out [their README](https://github.com/rust-lang-nursery/rustfmt) for details.


### Common Abbreviations

`ST`: Sql Type. Basically always has the `NativeSqlType` constraint

`DB`: Database. Basically always has the `Backend` constraint.

`QS`: Query Source. Usually doesn't have a constraint, but sometimes will have `QuerySource` attached

`PK`: Primary Key

`Lhs`: Left Hand Side

`Rhs`: Right Hand Side

`Conn`: Connection

Generally, we prefer to give our types meaningful names. `Lhs` and `Rhs` vs `T` and `U` for a binary expression, for example.
