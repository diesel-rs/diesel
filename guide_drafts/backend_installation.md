# Installing Backend Client Libraries

Diesel supports SQLite, PostgreSQL, and MySQL as database backends.
By default, `diesel_cli`
requires the client library of all three backends to be installed.
If one is missing, then `cargo install diesel_cli` you will throw an error like:

```
note: ld: library not found for -lmysqlclient
clang: error: linker command failed with exit code 1 (use -v to see invocation)
```

To install `diesel_cli` without all backends,
specify `--no-default-features`.
Use cargo's `--features` option to specify `postgres`, `sqlite`, and/or `mysql`.
For example, to install with sqlite only, run:

```
cargo install diesel_cli --no-default-features --features sqlite
```

For projects that depend on diesel,
you can specify which backends are required in the `Cargo.toml`.
For example:

```
[dependencies]
diesel = { version = "X.X.X", features = ["sqlite"] }
```

Below are commands to run
to install appropriate database clients
with various package managers.

## Debian/Ubuntu

### SQLite

`sudo apt-get install libsqlite3-dev`

### PostgreSQL

`sudo apt-get install postgresql-server-dev-all`

### MySQL

1. Install the following to add the MySQL APT repository. 

    ```
    wget https://dev.mysql.com/get/mysql-apt-config_0.8.7-1_all.deb
    sudo dpkg -i mysql-apt-config_0.8.7-1_all.deb
    ```

    Select `<Ok>`.

2. Retrieve new lists of packages

    ```
    sudo apt-get update
    ```

3. Install the client library

    ```
    sudo apt-get install libmysqlclient-dev
    ```

See [MySQL docs](https://dev.mysql.com/doc/mysql-apt-repo-quick-guide/en/) for more details.

## CentOS/Fedora
### SQLite

`sudo yum install sqlite-devel`

### PostgreSQL

`sudo yum install postgresql-devel`

### MySQL

`sudo yum install mysql-devel`

## Arch
### SQLite

`sudo pacman -Su sqlite`

### PostgreSQL

`sudo pacman -Su postgresql`

### MySQL

`sudo pacman -Su mysql`

## Mac OSX 
### SQLite
Already installed by default.
### PostgreSQL

`brew install postgresql`

### MySQL

`brew install mysql`
