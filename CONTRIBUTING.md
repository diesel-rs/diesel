# Contributing

Thanks for your interest in contributing to Diesel! We very much look forward to
your suggestions, bug reports, and pull requests.

We run an active [discussion forum](https://github.com/diesel-rs/diesel/discussions) where you can ask Diesel-related questions and
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

Diesel's issue tracker is meant to represent our current roadmap. An open issue represents either a bug, or a new feature that a member of the Diesel team is actively working on.

This means that you should not submit a feature request to our issue tracker, unless you were asked to do so by a member of the Diesel team. Feature requests should instead be posted in
our [discussion forum](https://github.com/diesel-rs/diesel/discussions/categories/ideas).

If you can't find thread describing your idea on our forum, create a new one. Adding answers to the following questions in your description is +1:

-   What do you want to do, and how do you expect Diesel to support you with that?
-   How might this be added to Diesel?
-   What are possible alternatives?
-   Are there any disadvantages?

Thank you! We'll try to respond as quickly as possible.

## Improve the documentation

We are welcoming contributions that improve the documentation, examples or the guides provided on the web page. 
These contribution are as valuable as any code contribution. So if you notice something that could be documented
in a better way or that is missing an example do not hesitate to open a PR to improve the documentation for all users.

## Triaging issues & Reviewing changes

The Diesel project receives a significant number of bug reports and pull requests. Any help reviewing and classifying these reports are highly welcome. For PR's you can just leave review comments. Otherwise you are welcome to join the [Diesel Reviewer team](https://github.com/orgs/diesel-rs/teams/reviewers) by requesting access [in this issue](https://github.com/diesel-rs/diesel/issues/1186). Members of this team get pinged on PR's that need a review and do have the right to triage issues. Especially PR reviews are a good place to become more familiar with certain Rust idioms and Diesel internals as they are a good place to ask questions about how something works.

## Contribute code to Diesel

We try to keep a number of issues [marked as good first issue](https://github.com/diesel-rs/diesel/issues?q=is%3Aissue%20state%3Aopen%20label%3A%22good%20first%20issue%22%20label%3A%22help%20wanted%22%20label%3A%22mentoring%20available%22) in our issue tracker. These are usually a good starting point if you are new to contribute to Diesel. We also keep a project to [plan](https://github.com/orgs/diesel-rs/projects/1) features for the next Diesel release. Feel free to grab any open issue in our tracker or project tracking by leaving a comment there. Also do not hesitate to ask for help if you are stuck trying to resolve a specific issue. Other contributors usually can help you around most problems.

### Setting up Diesel locally

1. Install Rust using [rustup], which allows you to easily switch between Rust
   versions. Diesel currently supports Rust Stable, Nightly, Rust Beta.

2. Install the system libraries needed to interface with the database systems
   you wish to use.

   These are the same as when compiling Diesel. It's generally a good idea
   to install _all_ drivers so you can run all tests locally.

   *Shortcut:* On macOS, you don't need to install anything to work with SQLite.
   For PostgreSQL, you'll only need the server (`libpq` is installed by
   default). To get started, `brew install postgresql@17 mysql` and follow the
   instructions shown to set up the database servers. Other versions of
   PostgreSQL should work as well.
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

   If you have [Docker](https://www.docker.com/), the following snippet might help you
   to get Postgres and MySQL running (with the above `.env` file):

   ```bash
   #!/usr/bin/env sh
   set -e
   docker run -d --name diesel.mysql -p 3306:3306 -e MYSQL_ALLOW_EMPTY_PASSWORD=true mysql
   while
     sleep 1;
     ! echo 'CREATE DATABASE diesel_test; CREATE DATABASE diesel_unit_test;' | docker exec -i diesel.mysql mysql
   do sleep 1; done

   docker run -d --name diesel.postgres -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres
   while
     sleep 1;
     ! echo 'CREATE DATABASE diesel_test;' | docker exec -i diesel.postgres psql -U postgres
   do :; done
   ```

   If you want to use docker-compose, you can execute docker-compose command like this.

    ```bash
    $ docker-compose up
    ```
    
5. Install [cargo-nextest](https://nexte.st/) via `cargo install cargo-nextest`

6. Now, try running the test suite to confirm everything works for you locally
   by executing `cargo xtask run-tests`. (Initially, this will take a while to compile
   everything.) In addition, if you want to compile and test a crate separately, 
   you can refer to the commands printed and executed by `cargo xtask run-tests`. Additionally you 
   can check `cargo xtask run-tests --help` on how to further configure which tests are executed.

[rustup]: https://rustup.rs/

### Coding Style

We follow the [Rust Style Guide](https://github.com/rust-dev-tools/fmt-rfcs/blob/master/guide/guide.md), enforced using [rustfmt](https://github.com/rust-lang/rustfmt).
To run rustfmt tests locally:

1. Use rustup to set rust toolchain to the version specified in the
   [rust-toolchain file](./rust-toolchain).

2. Install the rustfmt and clippy by running
   ```
   rustup component add rustfmt
   rustup component add clippy
   ```

3. Install [typos](https://github.com/crate-ci/typos) via `cargo install typos-cli`

4. Use `cargo xtask tidy` to check if your changes follow the expected code style.
   This will run `cargo fmt --check`, `typos` and `cargo clippy` internally. See `cargo xtask tidy --help`
   for additional options.

You can also use rustfmt to make corrections or highlight issues in your editor.
Check out [their README](https://github.com/rust-lang/rustfmt) for details.

### Common Abbreviations

`ST`: Sql Type. Basically always has the `NativeSqlType` constraint

`DB`: Database. Basically always has the `Backend` constraint.

`QS`: Query Source. Usually doesn't have a constraint, but sometimes will have `QuerySource` attached

`PK`: Primary Key

`Lhs`: Left Hand Side

`Rhs`: Right Hand Side

`Conn`: Connection

Generally, we prefer to give our types meaningful names. `Lhs` and `Rhs` vs `T` and `U` for a binary expression, for example.

### Compile Tests

Diesel has an extensive suite of compile tests in the `diesel_compile_tests` crate. These test work by having a small test program for each test case and then verifying that the compilation of those tests fail with a specific error message. For that we use the [`ui_test`](https://docs.rs/ui_test/latest/ui_test/) also used by rustc.  
Running these tests can done by simply running `cargo test` in the `diesel_compile_tests` directory. Adding new tests simply requires adding a new file to `diesel_compile_tests/tests/fail/` containing the source code you want to test.
You can run these tests with the environment variable `BLESS` set to `1` to update the expected stderr output. You also need to update the inline error annotations in the source code to match on the error message. See the documentation of `ui_test` for how to do that. 

### Snapshot tests

Diesel's test suite is using [insta](https://docs.rs/insta/latest/insta/) for snapshot tests in various places. If you get an error in the test suite that some output of such a test changed you can use [cargo-insta](https://docs.rs/insta/latest/insta/) to review and accept these changes. You need to commit these changes as part of your changeset.

Such snapshot tests are used by the following tests:

* Expanded code tests in `diesel_derives`
* Print-schema tests in `diesel_cli`
* Generate-migration tests in `diesel_cli`
