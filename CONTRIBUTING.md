# Contributing

Thanks for wanting to contribute to Diesel! We are very much looking forward to
your suggestions, bug reports, and pull requests.

Aside from Github, we have an active [Gitter
channel](https://gitter.im/diesel-rs/diesel), where you can ask questions and
get help on how to use Diesel. Feel free to ask there before opening an issue or
pull request on Github.

*Remember:* Anyone who interacts with Diesel in any space including but not
limited to this GitHub repository is expected to follow our [code of
conduct](https://github.com/diesel-rs/diesel/blob/master/code_of_conduct.md).


## Submitting bug reports

Have a look at our [issue tracker]. If you can't find an issue (open or closed)
describing your problem (or a very similar one) there, please open a issue with
the following details:

- Which versions of Rust and Diesel are you using?
- Which feature flags are you using?
- What are you trying to accomplish?
- What is the full error you are seeing?
- How can we reproduce this?
  - Please quote as much of your code as needed to reproduce (best link to a
    public repository or a [Gist])
  - Please post as much of your database schema as is relevant to your error

[issue tracker]: https://github.com/diesel-rs/diesel/issues
[Gist]: https://gist.github.com

Thank you! We'll try to get back to you as soon as possible.


## Submitting feature requests

If you can't find an issue (open or closed) describing your idea on our [issue
tracker], open an issue. It would be great if you could answer the following
questions in your description:

- What do you want to do and how do you expect Diesel to support you with that?
- How do you think this can be added to Diesel?
- What are possible alternatives?
- Are there any disadvantages?

Thank you! We'll try to get back to you as soon as possible.


## Contribute code to Diesel

### Setting up Diesel locally

1. Install Rust using [rustup], which allows you to easily switch between Rust
   versions. Diesel currently supports Rust Stable, Nightly, Rust Beta.

   If you want to run Diesel's test suite with _all_ supported features (extra
   lints and compiletest), you should use the same nightly version as Diesel's
   continuous integration. You can find it by looking for a line like
   `rust: nightly-2017-06-06` in the `.travis.yml` file. You can install and
   set a custom nightly version for a project using
   `rustup override add nightly-2017-06-06`.

2. Install the system libraries needed to interface with the database systems
   you wish to use.

   These are the same as when compiling diesel. In general, it is a good idea
   to have _all_ drivers installed so you can run all tests locally.

   *Shortcut:* On macOS, you don't need to install anything to work with SQLite
   and for PostgreSQL you'll only need the server (`libpq` is installed by
   default). So, to get started, `brew install postgresql mysql` and follow the
   instructions shown to set up the database servers.
3. Clone this repository and open it in your favorite editor.
4. Create a `.env` file in this directory, and add the connection details for
   your databases.

   See [.env.sample](.env.sample) for an example that should work with a trivial
   local setup.

   *Note:* If you didn't specify the MySQL user to be one with elevated
   permissions, you'll want to a command like ```mysql -c "GRANT ALL ON
   `diesel_%`.* TO ''@'localhost';" -uroot```, or something similar for the
   user that you've specified.

   If you have [Docker](https://docker.io), the following snippet might be
   useful to get Postgres and MySQL running (with the above `.env` file):

   ```bash
   #!/usr/bin/env sh
   set -e
   docker run -d --name diesel.mysql -p 3306:3306 -e MYSQL_ALLOW_EMPTY_PASSWORD=true mysql
   while
     sleep 1;
     ! echo 'CREATE DATABASE diesel_test;' | docker exec -i diesel.mysql mysql
   do sleep 1; done

   docker run -d --name diesel.postgres -p 5432:5432 postgres
   while
     sleep 1;
     ! echo 'CREATE DATABASE diesel_test;' | docker exec -i diesel.postgres psql -U postgres
   do :; done
   ```
5. Now, try running the test suite to confirm everything works for you locally
   by executing `bin/test`. (Initially, this will take a while to compile
   everything.)

[rustup]: https://www.rustup.rs
