# Custom Array and Data Types in Postgres

Table of Content:

1. [Concepts](#concepts)
2. [The project](#the-project)
3. [Getting started](#getting-started)
4. [Postgres schema](#postgres-schema)
5. [Postgres Enum](#postgres-enum)
6. [Postgres custom type](#postgres-custom-type)
7. [Postgres SERIAL vs INTEGER primary key](#postgres-serial-vs-integer-primary-key)
8. [Postgres array with custom type](#postgres-array-with-custom-type)
9. [Rust Types](#rust-types)
10. [Additional methods](#additional-methods)
11. [Applied Best Practices](#applied-best-practices)
12. [Testing](#testing)

In this guide, you learn more about the concepts listed below and illustrate the actual usage in Diesel with a sample
project.

## Concepts:

* Custom Enum
* Custom type that uses a custom Enum
* Array of custom types
* Serialization / Deserialization of custom type array
* Implementation of DB operations on a Rust Type

## The project:

Your company decided to create a database of all microservices in its system; the high-level requirements are:

* CRUD: Create, read, update, and delete a service in the database
* List all offline services and all services that are online
* Retrieve all API endpoints for a service

Upon closer inspection, you realize a few missing details in the requirements:

* There are many different endpoint types, some are http, some are gRPC, and some are even UDP (i.e. a message bus), but
  the number of endpoint types is limited to just three that haven’t changed in years.
* While each service may have one or more endpoints, no two services can share the same API endpoints, which means
  normalizing API endpoints in a separate table makes little sense in this case.
* Unmentioned in the requirements, a service depends on other services, and it would be great to test, before deploying
  a new service, if all of its dependencies are online.
* Since this part of the database contains only meta-data, its best to store it in a separate schema to separate the
  metadata from all other data.

Thinking further, you realize a few things:

1) Endpoint type can be expressed as an Enum in Postgres and in Rust
2) Endpoint can be a custom type in Postgres that matches a Rust type
3) Service contains an array of custom type Enum
4) When each service gets a unique ID that also serves as primary key, then service dependencies are basically just a
   collection of those unique service IDs. Thus, you just add an integer array in Postgres.
5) Checking all dependencies of a service is then as simple as loading the array of ID’s, iterating over it, check each
   dependency if its online, and you are basically done with the core requirement. As the old saying goes, if you can
   solve a problem with data structures, by all means, just do it.

## Getting started

Let’s crate a new crate, called custom_arrays:

`
cargo new custom_arrays –lib
`

Next, run the Diesel setup:

```bash 
diesel setup
```

And then generated a new Diesel migration called services:

```bash 
diesel migration generate services
```

This creates a new folder within the migration folder containing an empty up.sql and down.sql file.

## Postgres schema

Since the service management operates independently from the rest of the system, it is sensible to store all data in a
dedicated schema. A schema in Postgres is like a namespace as it ensures unique table and type names within the schema.
More importantly, you can create a new user that specifically has access to only a particular schema and that is always
a good security practice. To create a new schema with Diesel, you follow three steps:

1) Declare the schema name in the diesel.toml file
2) Declare the schema in the migration up/down.sql files
3) Add the schema to the SQL type as annotations

Postgres uses internally a schema as something like a search path for a table and, by default, searches in the public
schema for a table. If that fails, Postgres returns an error that the table does not exists (in the default schema).
Because of the search path mechanism, you only have to declare the schema name and consistently prefix the tables in the
schema with the schema name and that’s it.

In your diesel.toml file, add the following entry:

```toml
[print_schema]
file = "src/schema.rs"
custom_type_derives = ["diesel::query_builder::QueryId", "Clone"]

schema = "smdb"
```

Specifically, in the created migration folder, add the following to your up.sql file

```sql
-- Your SQL goes here
CREATE SCHEMA IF NOT EXISTS smdb;
```

Also, add the corresponding drop operation in the down.sql file.

```sql
DROP SCHEMA IF EXISTS smdb;
```

## Postgres Enum

The company only uses three types of API endpoints, gRPC, http, or UDP. However, because data entry or transmission
errors happen, an UnknownProtocol has been added to catch everything else that may go wrong to ensure that a potential
serialization or deserialization bug does not crash the system.
Instead, every once a while a Cron job runs over the database and searches for
those UnknownProtocol entries, reports it to the admin who may fixes the incorrect entries. Therefore the
Postgres ENUM in your up.sql looks like this:

```sql
-- Your SQL goes here
CREATE SCHEMA IF NOT EXISTS smdb;

CREATE TYPE smdb.protocol_type AS ENUM (
    'UnknownProtocol',
    'GRPC',
    'HTTP',
    'UDP'
);
```

Notice the schema prefix before the Enum name.
Add the corresponding drop type operation to your down.sql file:

```sql
DROP TYPE IF EXISTS smdb.protocol_type CASCADE;
DROP schema IF EXISTS smdb;
```

## Postgres custom type

The service endpoint in this case is modelled as a custom type.
There are few cases when you want to choose a custom type over a table:

1) The data have no relation to data in other tables.
2) You want to group a set of fields i.e. to store a complex configuration file with nested fields.
3) You specifically don’t want relations because of specific requirements.

The service endpoint is an example of the first two cases.
For once, the endpoints have no relation to any other data therefore a separate table makes little sense. Secondly, the
entire service metadata database really is more of a configuration store and, in a way, the service table really is just
one complex configuration with nested fields.
Also, worth mentioning, if you were to store endpoints in a separate table, then you would have to deal with resolving
relations during query time and that is probably a bit too much for just loading a configuration.
Rather, when using the custom type, you just access a field that is basically a tuple.
With that out of the way,
your endpoint type looks like this in your up.sql:

```sql
-- Your SQL goes here

CREATE TYPE smdb.service_endpoint AS (
	"name" Text,
	"version" INTEGER,
	"base_uri" Text,
	"port" INTEGER,
	"protocol" smdb.protocol_type
);
```

Again, all custom types are prefixed with the custom schema.
And the matching drop operation in the down.sql file:

```sql
DROP TYPE IF EXISTS smdb.service_endpoint CASCADE;
DROP TYPE IF EXISTS smdb.protocol_type CASCADE;
DROP schema IF EXISTS smdb;
```

## Postgres SERIAL vs INTEGER primary key

There is an important detail you must decide upfront: Internal or External Primary key?

External primary key refers to the idea that you designate primary keys outside of Postgres.
Let’s think this through:

### Internal primary key: SERIAL type

When you only need a primary key as an index, you just use the SERIAL and Postgres gives your data an automatically
incrementing integer primary key.
You usually can return the primary key value after inserting so if you need to know the key, you do get it after insert.

### External primary key: INTEGER type

In case you need to know the specific value of the primary key and you need to know it before inserting the data, you
have to assign unique primary keys before inserting data and you have to use the INTEGER type (or any of the many integer
variations) to convey to Postgres that you set the primary key yourself. Notice, you still have to set the NOT NULL and
PRIMARY KEY attribute to ensure data consistency.

In case of the microservice database, I am afraid, you have to use external primary keys because, remember, a service
depends on other services. In order to insert a service that depends on any other service, regardless of whether it has
already been inserted or not, you have to know the service ID upfront. With that out of the way, let’s define the
service table.

## Postgres array with custom type

To define the service table, the add the following to your up.sql file

```sql
CREATE TABLE  smdb.service(
	"service_id" INTEGER NOT NULL PRIMARY KEY,
	"name" Text NOT NULL,
	"version" INTEGER NOT NULL,
	"online" BOOLEAN NOT NULL,
	"description" Text NOT NULL,
	"health_check_uri" Text NOT NULL,
	"base_uri" Text NOT NULL,
	"dependencies" INTEGER[] NOT NULL,
	"endpoints" smdb.service_endpoint[] NOT NULL
);
```

Add the matching drop statement to your down.sql file:

```sql
-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS smdb.service;
DROP TYPE IF EXISTS smdb.service_endpoint CASCADE;
DROP TYPE IF EXISTS smdb.protocol_type CASCADE;
DROP SCHEMA IF EXISTS smdb;
```

A few notes on the service table:

* All types and tables are prefixed with the custom schema to give Postgres a
  hint where to find these types and tables.
* The array type follows the usual convention schema.type[].
* The NOT NULL attribute means that the array itself must be set; Values inside the array still might be null.

The Diesel team decided that array values are nullable by default because there so many things that may go wrong when
dealing with array values thus making them nullable by default prevents a few potential crashes. Conversely, it also
means you have to unwrap option types whenever you access data deserialized from an array.

In total, the up.sql looks as below:

```sql
-- Your SQL goes here
CREATE SCHEMA IF NOT EXISTS smdb;

CREATE TYPE smdb.protocol_type AS ENUM (
    'UnknownProtocol',
    'GRPC',
    'HTTP',
    'UDP'
);

CREATE TYPE smdb.service_endpoint AS (
	"name" Text,
	"version" INTEGER,
	"base_uri" Text,
	"port" INTEGER,
	"protocol" smdb.protocol_type
);

CREATE TABLE  smdb.service(
	"service_id" INTEGER NOT NULL PRIMARY KEY,
	"name" Text NOT NULL,
	"version" INTEGER NOT NULL,
	"online" BOOLEAN NOT NULL,
	"description" Text NOT NULL,
	"health_check_uri" Text NOT NULL,
	"base_uri" Text NOT NULL,
	"dependencies" INTEGER[] NOT NULL,
	"endpoints" smdb.service_endpoint[] NOT NULL
);
```

Now, it’s time to run the Diesel migration:

```bash 
diesel migration run
```

You may use a database console to double check if all types and tables have been created inside the custom schema. If a
type somehow appears in the public default schema, double check if the type definition in up.sql has been prefixed with
the schema name.

The Diesel migration generates the following schema.rs file in the src root:

```rust
// @generated automatically by Diesel CLI.

pub mod smdb {
    pub mod sql_types {
        #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
        #[diesel(postgres_type(name = "service_endpoint", schema = "smdb"))]
        pub struct ServiceEndpoint;
    }

    diesel::table! {
        use diesel::sql_types::*;
        use super::sql_types::ServiceEndpoint;

        smdb.service (service_id) {
            service_id -> Int4,
            name -> Text,
            version -> Int4,
            online -> Bool,
            description -> Text,
            health_check_uri -> Text,
            base_uri -> Text,
            dependencies -> Array<Nullable<Int4>>,
            endpoints -> Array<Nullable<ServiceEndpoint>>,
        }
    }
}
```

Notice, the Postgres types are stored in Postgres therefore you are not seeing them in the generated schema. Only tables
will show up in the generates schema. Furthermore, you will need a wrapper struct to map from Rust to the Postgres type.
For the ServiceEndpoint, Diesel already generated a matching zero sized SQL type struct with the correct annotations. The service
table then uses that SQL type `ServiceEndpoint` instead of the native Postgres type to reference the custom type
in the endpoints array.

You want to attach the generated schema.rs to your lib file to access it from within your crate and stop your IDE from
showing related warnings

Next, you want to create a folder model with a mod file in it attached to your lib file. This is your place to store all
Rust type implementations matching all the generated Postgres types and tables.

Lastly, you also want to add a type alias for a postgres database connection in the lib file because that gives you the
freedom to swap between a normal single connection or a connection pool without updating your type implementation or
tests.
The connection pool type alias has been uncommented, but the type signature of the pooled connection makes it obvious why
a type alias is a good idea.

At this point, your lib file looks as shown below:

```rust
mod schema;
pub mod model;

// Alias for a pooled connection.
// pub type Connection = diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::pg::PgConnection>>;

// Alias for a normal, single, connection.
pub type Connection = PgConnection;

```

## Rust Types

In total, you need implement three Rust types:

* Enum: ProtocolType
* Struct: Endpoint
* Struct: Service

### Rust Enum: ProtocolType

Diesel needs a zero sized SQL type struct to represent a custom Enum in Postgres, so let’s add one:

```rust
#[derive(SqlType)]
#[diesel(postgres_type(name = "protocol_type", schema = "smdb"))]
pub struct PgProtocolType;
```

It is important to add the database schema ("smdb") and type name ("protocol_type") otherwise Postgres fails to find the type and aborts an
operation on that type.

Next, let’s define the actual Enum. Bear in mind to use the SQL type struct as sql_type on the Enum:

```rust
#[derive(Debug, Clone, FromSqlRow, AsExpression, PartialEq, Eq)]
#[diesel(sql_type = PgProtocolType)]
pub enum ProtocolType {
    UnknownProtocol,
    GRPC,
    HTTP,
    UDP,
}
```

It’s worth pointing out that you are dealing with three types in total:

1) protocol_type: The Postgres Enum type
2) PgProtocolType: The SQL type struct that refers to the Postgres Enum type
3) ProtocolType: The Rust Enum that maps to the SQL type struct PgProtocolType

Mixing any of those three types will result in a complicated Diesel trait error. However, these error messages are just
a convoluted way to say that the database type mismatches the Rust type. When you encounter a consulted trait error
message, make sure to check:

1) Do I have a SQL type struct?
2) Does my a SQL type struct derive SqlType?
3) Does my SQL type has a `#[diesel(postgres_type(_)]` attribute declared?
4) Is the  `#[diesel(postgres_type(_)]` attribute referring to the correct Postgres type?
5) Does my Rust type refers to the SQL type struct?

If all those checks pass and you still see errors, it’s most likely a serialization error.
To serialize and deserialize a custom Enum, you write a custom ToSql and FromSql implementation.

`ToSql` describes how to serialize a given rust type (Self) as sql side type (first generic argument) for a specific
database backend (second generic argument).
It needs to translate the type into the relevant wire protocol in that case.
For postgres/mysql enums that's just the enum value as ascii string,
but in general it's depended on the database + type. Also, it is important
to end the implementation with Ok(IsNull::No).

```rust
impl ToSql<PgProtocolType, Pg> for ProtocolType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            ProtocolType::UnknownProtocol => out.write_all(b"UnknownProtocol")?,
            ProtocolType::GRPC => out.write_all(b"GRPC")?,
            ProtocolType::HTTP => out.write_all(b"HTTP")?,
            ProtocolType::UDP => out.write_all(b"UDP")?,
        }
        Ok(IsNull::No)
    }
}
```

`FromSql` describes how to deserialize a given rust type (Self) as sql side type (first generic argument) for a specific
database backend (second generic argument).
It need to translate from the relevant wire protocol to the rust type.
For postgres/mysql enums that just means matching on the as ascii string,
but in general it's depended on the database + type.

```rust
impl FromSql<PgProtocolType, Pg> for ProtocolType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"UnknownProtocol" => Ok(ProtocolType::UnknownProtocol),
            b"GRPC" => Ok(ProtocolType::GRPC),
            b"HTTP" => Ok(ProtocolType::HTTP),
            b"UDP" => Ok(ProtocolType::UDP),
            _ => Err(DatabaseError(
                DatabaseErrorKind::SerializationFailure,
                Box::new(format!(
                    "Unrecognized enum variant: {:?}",
                    String::from_utf8_lossy(bytes.as_bytes())
                )),
            )
            .into()),
        }
    }
}
```

In the from_sql, it is important to add error handling for unknown Enum variants
to catch errors that may result from incorrect database updates.

### Rust Struct Endpoint

```rust 
#[derive(Debug, Clone, FromSqlRow, AsExpression, PartialEq, Eq)]
#[diesel(sql_type = crate::schema::smdb::sql_types::ServiceEndpoint)]
pub struct Endpoint {
    pub name: String,
    pub version: i32,
    pub base_uri: String,
    pub port: i32,
    pub protocol: ProtocolType,
}
```

It is worth mentioning is that the sql_type refers to the sql side type generated by Diesel and stored in the schema.rs
file.
Keep this in mind if you ever refactor the schema.rs file into a different folder because the macro annotation checks
the path during compilation and throws an error if it cannot find the type in the provided path.
The serialize and deserialize implementation of a custom type is not as obvious as the Enum because, internally,
Postgres represent a custom type as an anonymous typed tuple. Therefore, you have to map a struct to a typed tuple and
back.

Let’s start with the ToSql implementation to store the Rust Endpoint struct as typed tuple.
Luckily, Diesel provides a helper util to do just that.

```rust 
impl ToSql<crate::schema::smdb::sql_types::ServiceEndpoint, Pg> for Endpoint {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        serialize::WriteTuple::<(Text, Integer, Text, Integer, PgProtocolType)>::write_tuple(
            &(
                self.name.to_owned(),
                self.version.to_owned(),
                self.base_uri.to_owned(),
                self.port.to_owned(),
                self.protocol.to_owned(),
            ),
            &mut out.reborrow(),
        )
    }
}
```

I cannot stress enough that it is paramount that the tuple type signature must match exactly the Postgres type signature
defined in your up.sql.
Ideally, you want to use the split view function in your IDE to have the up.sql in one pane and the the ToSql
implementation in another pane, both side by side, to double check
that the number and types match.
If the type or number of types mismatch, you will get a compiler error telling you that somehow either
the number of fields don’t match or that the type of the fields don’t match.
Also, because the write_tuple expects values, you have to call either to_owned() or clone() on any referenced data.

The FromSql reverses the process by converting a Postgres typed tuple back into a Rust struct.
Again, Diesel provides a convenient helper to do so:

```rust 
impl FromSql<crate::schema::smdb::sql_types::ServiceEndpoint, Pg> for Endpoint {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let (name, version, base_uri, port, protocol) =
            FromSql::<Record<(Text, Integer, Text, Integer, PgProtocolType)>, Pg>::from_sql(bytes)?;

        Ok(Endpoint {
            name,
            version,
            base_uri,
            port,
            protocol,
        })
    }
}
```

Similar to the serialization process, it is paramount that the type annotation
match exactly the one used in ToSql and the type definition in up.sql.

Debugging serialization issues in Diesel is relatively straight forward
since the compiler usually points out the location of the issue and, often, the issue is a type mismatch that is
relatively easy to fix.

It’s worth repeating that you are dealing with three types in total:

1) service_endpoint: The Postgres custom type
2) ServiceEndpoint: The SQL type struct generated by Diesel that refers to the Postgres type service_endpoint. Also, only
   the SQL type struct carries the postgres_type annotation.
3) Endpoint: The Rust struct with an sql_type annotation referring to the SQL type struct ServiceEndpoint

Make sure all of those types match correctly.

### Rust Struct Service

The service struct gets its serialization and deserialization implementation generated by Diesel so that saves some
typing. On the other hand, it is a good practice to implement database operations on the actual type itself.
The wisdom here is twofold. For once, you cannot separate the database operation from the type,
therefore it makes sense to implement the operations on the type itself.
This also hides implementation details in encapsulation,
as [discussed in the book](https://doc.rust-lang.org/book/ch17-01-what-is-oo.html#encapsulation-that-hides-implementation-details).

Second, you gain a single point of maintenance because, in case your table definition changes, you have to update the
Rust type anyways and because Diesel is statically checked, the compiler will immediately point out where your
operations need to be corrected to match the new type definition.

The requirements stated we want CRUD operations, and type-based programming suggest creating a different type per
operation. Because we use external primary keys, we don’t need an additional type for the delete operation, therefore,
in total, we crate 3 service types:

1) Service: For Read
2) CreateService: For create operations
3) UpdateService: For update operations

The Service and CreateService have identical fields.
Also, both types require a primary key annotation in addition to
the table name. For more details about inserts, refer to the
official [all about inserts guide](https://diesel.rs/guides/all-about-inserts.html).

The UpdateService, however, has each field wrapped into an option type. When the option type is Some, Diesel knows that
this field needs updating. If the field is set to None, Diesel ignores it.
For more details about updates, refer to the
official [all about updates guide](https://diesel.rs/guides/all-about-updates.html)

The relevant type declarations are:

```rust
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name= crate::schema::smdb::service,  primary_key(service_id))]
pub struct Service {
    pub service_id: i32,
    pub name: String,
    pub version: i32,
    pub online: bool,
    pub description: String,
    pub health_check_uri: String,
    pub base_uri: String,
    pub dependencies: Vec<Option<i32>>,
    pub endpoints: Vec<Option<Endpoint>>,
}

#[derive(Debug, Clone, Queryable, Insertable)]
#[diesel(table_name= crate::schema::smdb::service,  primary_key(service_id))]
pub struct CreateService {
    pub service_id: i32,
    // ... skipped
    pub endpoints: Vec<Option<Endpoint>>,
}

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset)]
#[diesel(table_name= crate::schema::smdb::service)]
pub struct UpdateService {
    pub name: Option<String>,
    pub version: Option<i32>,
    pub online: Option<bool>,
    pub description: Option<String>,
    pub health_check_uri: Option<String>,
    pub base_uri: Option<String>,
    pub dependencies: Option<Vec<Option<i32>>>,
    pub endpoints: Option<Vec<Option<Endpoint>>>,
}
```

Next, let’s implement the CRUD operations on the service type.
Remember the handy connection type alias defined earlier?
This service implementation is the place to use it.

### **Create**

```rust
impl Service {
    pub fn create(db: &mut Connection, item: &CreateService) -> QueryResult<Self> {
        insert_into(crate::schema::smdb::service::table)
            .values(item)
            .get_result::<Service>(db)
    }

}
```

The insert into function needs the table as target and the value to insert. It is a common convention to return the
inserted value so that the callsite can verify that the insert completed correctly.

### **Read**

```rust
    pub fn read(db: &mut Connection, param_service_id: i32) -> QueryResult<Self> {
        service
            .filter(service_id.eq(param_service_id))
            .first::<Service>(db)
    }
```

Each service has one unique ID and therefore querying for a service ID returns either exactly one result or an error.
That means, the read operation completes with capturing and returning the first result. In case you have data that may
return multiple entries for a search key, you must change the return type of the read method to QueryResult<vec<MyType>>
and use load instead of first, just as it shown in the read all method.

### **Read All**

```rust
    pub fn read_all(db: &mut Connection) -> QueryResult<Vec<Self>> {
        service.load::<Service>(db)
    }
```

Here, we just load everything from the service table and return the entire result as a vector. Note, it is expected that
the service table only has relatively few entries. For tables that hold a large number of data, you can implement
pagination, add limit clause, or some kind of filtering to reduce the number of returned values.

### **Update**

```rust
    pub fn update(
        db: &mut Connection,
        param_service_id: i32,
        item: &UpdateService,
    ) -> QueryResult<Self> {
        diesel::update(service.filter(service_id.eq(param_service_id)))
            .set(item)
            .returning(Service::as_returning())
            .get_result(db)
    }
```

The update method, similar to insert, requires the target table and update values as argument and returns the updated
service as result. Notice, the parameter is of type UpdateService to ensure compiler verification.

### **Delete**

```rust
    pub fn delete(db: &mut Connection, param_service_id: i32) -> QueryResult<usize> {
        diesel::delete(service.filter(service_id.eq(param_service_id))).execute(db)
    }
```

Delete is a standard function provided by Diesel. Notice, the filter searched over the primary key to ensure only one
service with the matching ID gets deleted. However, if you were to filter over a non-unique attribute, you may end up
deleting more data than you though you would. In that case, always run the corresponding SQL query in a SQL console to
double check that your filter criterion returns exactly what you want it to return.

With the CRUD methods implemented, it’s time to look at the more interesting part of the requirements.

## Additional methods

In the requirement, it was stated that:

“... a service depends on other services, and it would be great to test before deploying a
new service, if all of its dependencies are online.”

To do so we have to implement a method that sets a service online and another one to set it offline. By experience, a
few
non-trivial errors are caused by incorrectly set Boolean flags and whenever you can, hide Boolean flags in your public
API and use a dedicated method instead to the set them.

Next, we need a method to return true if a service is online. At this point, we also want to add a method that checks if
a service actually is in the database. Imagine, you want to test if you can deploy a new service with, say 5
dependencies, and somehow the database returns no, you can’t. The obvious question is why? There might be some services
that are offline, fair enough, just start them, but it might also be possible that somehow a service isn’t currently in
the database and that is where you need another check method.

Then, we want another method that takes a collection of service ID’s, checks all of them and returns either true if all
services are online, or returns an error message that states which service is offline.

Finally, let’s add another method that returns all services that are online and another one that
returns all services that are currently offline. The latter is handy to quickly identify a deployment blocker.

### Set Service Online / Offline

Let’s start with a private helper method that sets the Boolean online flag on a service in the database.

```rust 
    fn set_svc_online(
        db: &mut Connection,
        param_service_id: i32,
        param_online: bool,
    ) -> QueryResult<()> {
        match diesel::update(service.filter(service_id.eq(param_service_id)))
            .set(online.eq(param_online))
            .returning(Service::as_returning())
            .get_result(db)
        {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
```

Here the generated DSL really shines as you can easily select a field and set it to a new value. Notice, the return type
is void in case of success. Realize, if you were to return the updated Boolean flag, you would cause a non-trivial
ambiguity on the call-site. For setting the Boolean flag true, the result would be true, which is correct, but there is
also a risk of the return value being interpreted as the operation having succeeded. And that matters, because if you
set the flag to false, the same method return false, which is also correct, but could be misunderstood as the operation
didn’t completed because false was returned. A good API is mostly free of ambiguity and therefore, therefore you just
return Ok(()) in case of success because there is no way to misunderstand a returned ok.

Now, let’s add two public wrapper methods that set the flag to either true or false.

```rust 
    pub fn set_service_online(db: &mut Connection, param_service_id: i32) -> QueryResult<()> {
        Self::set_svc_online(db, param_service_id, true)
    }

    pub fn set_service_offline(db: &mut Connection, param_service_id: i32) -> QueryResult<()> {
        Self::set_svc_online(db, param_service_id, false)
    }
```

### Check Service Exists

Next up, we add a method to check if service is in the database and the easiest way to implement this is to test if find
returns a result or error.

```rust
    pub fn check_if_service_id_exists(
        db: &mut Connection,
        param_service_id: i32,
    ) -> QueryResult<bool> {
        match service.find(param_service_id).first::<Service>(db) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
```

### Check Service Online

To check if a service is online, we again lean on the generated DSL:

```rust
    pub fn check_if_service_id_exists(
        db: &mut Connection,
        param_service_id: i32,
    ) -> QueryResult<bool> {
        match service.find(param_service_id).first::<Service>(db) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
```

### Get Service Dependencies

Notice, if we select dependencies instead of online, we return all the dependencies of a service. Let’s add another
method to do just that:

```rust
    pub fn get_all_service_dependencies(
        db: &mut Connection,
        param_service_id: i32,
    ) -> QueryResult<Vec<Option<i32>>> {
        service
            .filter(service_id.eq(param_service_id))
            .select(dependencies)
            .first::<Vec<Option<i32>>>(db)
    }
```

### Get Service Endpoints

And likewise, if we select endpoints, we get all API endpoints of a service:

```rust
    pub fn get_all_service_endpoints(
        db: &mut Connection,
        param_service_id: i32,
    ) -> QueryResult<Vec<Option<Endpoint>>> {
        service
            .filter(service_id.eq(param_service_id))
            .select(endpoints)
            .first::<Vec<Option<Endpoint>>>(db)
    }
```

As an observation, when we replace the filter criterion with a test if a service is online, we get all services with
online set to true and that is very easy to implement:

### Get All Online Services

```rust
    pub fn get_all_online_services(db: &mut Connection) -> QueryResult<Vec<Self>> {
        service
            .filter(online.eq(true))
            .select(Service::as_returning())
            .load::<Service>(db)
    }
```

### Get All Offline Services

Likewise, we use the same filter to
find and return all services with online set to false.

```rust
    pub fn get_all_offline_services(db: &mut Connection) -> QueryResult<Vec<Self>> {
        service
            .filter(online.eq(false))
            .select(Service::as_returning())
            .load::<Service>(db)
    }
```

### Check All Services Online

Finally, we can implement the last method that takes a collection of service ID’s, checks all of them, and returns
either true, or the service ID that is offline.

```rust
    pub fn check_all_services_online(
        db: &mut Connection,
        services: &[i32],
    ) -> QueryResult<(bool, Option<String>)> {
        for id in services {
            match Service::check_if_service_id_online(db, *id) {
                Ok(res) => {
                    if !res {
                        return Ok((false, Some(format!("Service {} is offline", id))));
                    }
                }
                Err(e) => return Err(e),
            };
        }

        Ok((true, None))
    }
```

Here, it is a matter of taste to return a Boolean flag with an optional string or to encode the case of an offline
service as a custom error. The rational for the Boolean flag is that any service might be offline for any reason so that
doesn’t count as a database error whereas if the query would fail, that would amount to a database error. That said, the
decision depends largely on the design requirements and if you already have custom errors defined, then adding a
ServiceDependencyOfflineError variant should be straight forward.

At this point, the Service type implementation is complete.

## Applied Best Practices

Implementing the database operations on the Service type is simple, straight forward,
and follows three best practices:

1) Take the database connection as a parameter
2) Prefer the generated DSL over custom queries whenever possible
3) Add type annotations on database return types

### Database connection as a parameter

The first one refers to the idea that no type should hold application state, and a database connection definitely counts
as application state. Instead, you would write a database connection manager
that manages a Postgres connection pool, and then calls into the database methods implemented
in the service type giving it a pooled connection as parameter.

### Prefer the generated DSL over custom queries

The second one refers to the idea that non-complex queries are usually easier and more composable expressed in the
generated DSL than in custom SQL queries and therefore the DSL is preferred in that case. However, if you have to write
complex transactions, there is a chance the DSL isn’t cutting it, so in that scenario you may end up writing custom SQL.
That said, the DSL gets you a long way before you are writing custom SQL.

### Add type annotations

The last one, type annotation on database return types, isn’t that obvious because Rust can infer the database return
type from the method return type, so why bother? Why would you write get_result::<Service>(db) instead of just
get_result(db) you may wonder? Here is the big secret, type inference in Rust needs occasionally a hint. When you
annotated the database return type and then wants to daisy chain another operation via map or flatmap, without the type
annotation you get a compile error. If you have the type annotation in place, you can add more operations whenever you
want and things compile right away.
To illustrate the last point, the count operator in Postgres only returns an i64 integer. Suppose your application
expects an u64 from the persistence API. Wouldn’t it be nice to make the conversion on the fly?

Let’s take a closer look. Conventionally, you write the count operation like so:

```rust
  pub fn count(db: &mut Connection) -> QueryResult<i64> {
        service.count().get_result::<i64>(db)
    }
```

Notice, the get result has a type annotation for i64. If you remove it, the code still compiles. But leave it there for
a moment. Let’s add a map operation that takes an i64 returned from the count query and converts it into an u64. Notice,
we have to update the return type of the method as well.

```rust
   pub fn count_u64(db: &mut Connection) -> QueryResult<u64> {
        service.count()
        .get_result::<i64>(db)
        .map(|c| c as u64)
    }
```

Run the compile and see that it works. Now, if you were to remove the type annotation <i64> from get_result,
you get a compile error saying that trait FromSql is not implemented.
The type annotation is required because there can be more than one FromSql impl for the same rust type and there can be
more than one FromSql type for the same sql type. That gives a lot flexibility how you actually map data between your
database and your rust code, but it requires explicit type annotation.

If you write a database layer for an existing system, this technique comes in handy as you can seamlessly convert
between Diesel DB types and your target types while leaving your target types as it. And because you never know when you
have to do this, it’s generally recommended to add type annotations to DB return types.

## Testing

Diesel enables you to test your database schema and migration early on in the development process.
To do so, you need only meed:

* A util that creates a DB connection
* A util that runs the DB migration
* And your DB integration tests

### DB Connection Types

Broadly speaking, there are two ways to handle database connection. One way is to create one connection per application
and use it until the application shuts down. Another way is to create a pool of connection and,
whenever a database connection is needed for an DB operation, a connection is taken from the pool
and after the DB operation has been completed, the connection is returned to the pool.
The first approach is quite common in small application whereas the second one is commonly used in server application
that expected consistent database usage.

**Important**

Using database connection pool in Diesel requires you to enable the `r2d2` feature in your cargo.toml file.

In Diesel, you can handle connections either way, but the only noticeable difference is the actual connection type used
as type parameter. For that reason, a type alias for the DB connection was declared in the lib.rs file because
that would allow you to switch between a single connection and a pooled connection without refactoring.

### DB Connection Test Util

Let's start with a small util that returns a simple DB connection. First you need to get a database URI from somewhere,
then construct a connection, and lastly return the connection.
Remember, this is just a test util so there is no need to add anything more than necessary.

```rust
use dotenvy::dotenv;

fn postgres_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("POSTGRES_DATABASE_URL")
    .expect("POSTGRES_DATABASE_URL must be set");
    
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
```

Here we use the dotenv crate to test if there is an environment variable of either DATABASE_URL or POSTGRES_DATABASE_URL
and if so, parse the string and use it to establish a connection. Therefore, make sure the POSTGRES_DATABASE_URL is set
correctly.

### DB Migration Util

Diesel can run a database migration in one of two ways.
First, you can use the Diesel CLI in your terminal
to generate, run, or revert migrations manually. This is ideal for development when you frequently change the database
schema.

The second way is programmatically via the embedded migration macro,
which is ideal to build a single binary with all migrations compiled into it so that
you don't have to install and run the Diesel CLI on the target machine.
This simplifies deployment of your application and streamlines continuous integration testing.

**Important**

You must add the crate `diesel_migrations` to your cargo.toml and set the target database as feature flag to enable the
embedded migration macro.

To serve both purposes, deployment and CI testing, let's add a new function `run_db_migration` to the lib.rs file of the
crate that takes a connection as parameter, checks if the DB connection is valid, checks if there are any pending
migrations, and if so, runs all pending migrations. The implementation is straight forward, as you can see below:

```rust 
pub type Connection = PgConnection;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn run_db_migration(
    conn: &mut Connection,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    // Check DB connection!
    match conn.ping() {
        Ok(_) => {}
        Err(e) => {
            eprint!("[run_db_migration]: Error connecting to database: {}", e);
            return Err(Box::new(e));
        }
    }

    // Check if DB has pending migrations
    let has_pending = match conn.has_pending_migration(MIGRATIONS) {
        Ok(has_pending) => has_pending,
        Err(e) => {
            eprint!(
                "[run_db_migration]: Error checking for pending database migrations: {}",
                e
            );
            return Err(e);
        }
    };

    // If so, run all pending migrations.
    if has_pending {
        match conn.run_pending_migrations(MIGRATIONS) {
            Ok(_) => Ok(()),
            Err(e) => {
                eprint!("[run_db_migration]: Error migrating database: {}", e);
                Err(e)
            }
        }
    } else {
        // Nothing pending, just return
        Ok(())
    }
}
```

The rational to check for all potential errors is twofold. For once, because the database connection is given as a
parameter,
you just don't know if the component that created the connection has checked the connection, therefore you better check
it to catch a dead connection.
Second, even if you have a correct database connection, this does not guarantee that the migration will succeed.
There might be some types left from a previously aborted drop operations or
a random error might happened at any stage of the migration, therefore you have to handle the error where it occurs.
Also, because you run the db migration during application startup or before testing,
ensure you have clear error messages to speed up diagnostic and debugging.

### DB Integration Tests

Database integration tests become flaky when executed in parallel usually because of conflicting read / write
operations.
While modern database systems can handle concurrent data access, test tools with assertions not so much.
That means, test assertions start to fail seemingly randomly when executed concurrently.
There are only very few viable options to deal with this reality:

* Don't use parallel test execution
* Only parallelize test per isolated access
* Do synchronization and test the actual database state before each test and run tests as atomic transactions

Out of the three options, the last one will almost certainly win you an over-engineering award in the unlikely case
your colleagues appreciate the resulting test complexity. If not, good luck.
In practice, not using parallel test execution is often not possible either because of the larger number
of integration tests that run on a CI server. To be clear, strictly sequential tests is a great option
for small projects with low complexity, it just doesn't scale as the project grows in size and complexity.

And that leaves us only with the middle-ground of grouping tables into isolated access.
Suppose your database has 25 tables you are tasked to test.
Some of them are clearly unrelated, others only require read access to some tables,
and then you have those where you have to test for multi-table inserts and updates.

Say, you can form 7 groups of tables that are clearly independent of each other,
then you can run those 7 test groups in parallel, but for all practical purpose within each group,
all tests are run in sequence unless you are testing specifically for concurrent read write access.
You want to put those test into a dedicated test group anyways as they capture errors that are more complex
than errors from your basic integration tests.
And it makes sense to stage integration tests into simple functional tests, complex workflow tests,
and chaos tests that triggers read / write conflicts randomly to test for blind spots.

In any case, the test suite for the service example follow the sequential execution pattern so that they can be
executed in parallel along other test groups without causing randomly failing tests. Specifically,
the test structure looks as shown below. However, the full test suite is in the test folder.

```rust 
#[test]
fn test_service() {
    let mut connection = postgres_connection();
    let conn = &mut connection;

    println!("Test DB migration");
    test_db_migration(conn);

    println!("Test create!");
    test_create_service(conn);

    println!("Test count!");
    test_count_service(conn);
    
    //...
}    

fn test_db_migration(conn: &mut Connection) {
    let res = custom_arrays::run_db_migration(conn);
    //dbg!(&result);
    assert!(res.is_ok());
}
```

The idea here is simple yet powerful:
There is just one Rust test so regardless of whether you test with Cargo,
Nextest or any other test util, this test will run everything within it in sequence.
The print statements are usually only shown if a test fails.
However, there is one important details worth mentioning,
the assert macro often obfuscates the error message and if you have ever seen a complex stack trace full of fnOnce
invocations, you know that already.
To get a more meaningful error message, just uncomment the dbg!
statement that unwraps the result before the assertion and you will see a helpful error message in most cases.

You may have noticed that the DB migration util checks if there are pending migrations and if there is nothing, it does
nothing and just returns.
The wisdom behind this decision is that, there are certain corner cases
that only occur when you run a database tet multiple times and you really want to run the DB migration just once to
simulate that scenario as realistic as possible.
When you test locally, the same logic applies and you really only want to run a database migration when the schema has
changed. 

