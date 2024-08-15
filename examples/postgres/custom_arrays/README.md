# Custom array and custom data in Postgres

In this guide, you learn more about the concepts listed below and illustrate the actual usage in Diesel with a sample
project.

**Concepts**:

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
3) Prefix all table names in the migration up/down.sql files with the schema

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
DROP schema IF EXISTS smdb;
```

## Postgres Enum

The company only uses three types of API endpoints, gRPC, http, or UDP. However, because data entry or transmission
errors happen, an UnknownProtocol has been added to catch everything else that may go wrong to ensure no serialization /
deserialization bugs crash the system. Instead, every once a while a Chron job runs over the database and searches for
those UnknownProtocol entries, reports it to the admin who may fixes the incorrect entries one day. Therefore the
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

Notice, the table name is prefixed by the DB schema to ensure Postgres find it. Also, you add the corresponding drop
type operation to your down.sql file:

```sql
DROP TYPE IF EXISTS smdb.protocol_type CASCADE;
DROP schema IF EXISTS smdb;
```

## Postgres custom type

The service endpoint in this case is modelled as a custom type.
There are few cases when you want to choose a custom type over a table.

1) The data have no relation to data in other tables
2) You want to group a bunch of fields i.e. to store a configuration file with nested fields
3) You specifically don’t want to model relations

In the case of a service endpoint, it really is as simple as every service has one or more endpoints, but these are
certainly not worth storing in a separate table since then you would have to deal with resolving relations during query
time. Rather, when using the custom type, you just access a field that is basically a tuple. With that out of the way,
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
incrementing integer primary key. You usually can return the primary key value after inserting so if you need to know
the key, you do get it after insert.

### External primary key: INTEGER type

In case you need to know the specific value of the primary key and you need to know it before inserting the data, you
have to assign unique primary keys before inserting data and you have to use the INTEGE type (or any of the many integer
variations) to convey to Postgres that you set the primary key yourself. Notice, you still have to set the NOT NULL and
PRIMARY KEY attribute to ensure data consistency.

In case of the microservice database, I am afraid, you have to use external primary keys because, remember, a service
depends on other services. In order to insert a service that depends on any other service, regardless of whether it has
already been inserted or not, you have to know the service ID upfront. With that out of the way, let’s define the
service table.

## Postgres array with a custom type

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

And add the matching drop statement to your down.sql file:

```sql
-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS smdb.service;
DROP TYPE IF EXISTS smdb.service_endpoint CASCADE;
DROP TYPE IF EXISTS smdb.protocol_type CASCADE;
DROP schema IF EXISTS smdb;
```

A few notes on the service table:

* Notice the schema prefix appears in both, the table name and in referenced types
* The array type follows the usual convention type[]
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

This generates a schema.rs file in the src root:

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
For the ServiceEndpoint, Diesel already generated a matching wrapper struct with the correct annotations. The service
table then uses that wrapper struct “ServiceEndpoint” instead of the native Postgres type to reference the custom type
in the endpoints array.

You want to attach the generated schema.rs to your lib file to access it from within your crate and stop your IDE from
showing related warnings

Next, you want to create a folder model with a mod file in it attached to your lib file. This is your place to store all
Rust type implementations matching all the generated Postgres types and tables.

Lastly, you also want to add a type alias for a pooled connection in the lib file because that connection definition is
a long and convoluted type in need of a shorthand.

At this point, your lib file looks as shown below:

```rust
mod schema;
pub mod model;

pub type Connection = diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::pg::PgConnection>>;
```

## Rust Types

In total, you implement three Rust types:

* Enum: ProtocolType
* Struct: Endpoint
* Struct: Service

### Rust Enum: ProtocolType

Diesel needs a wrapper struct to store a custom Enum in Postgres, so let’s add one:

```rust
#[derive(SqlType)]
#[diesel(sql_type = protocol_type)]
#[diesel(postgres_type(name = "protocol_type"))]
pub struct PgProtocolType;
```

It is important that you add both, sql_type and postgres_type, otherwise insert will fail.

Next, let’s define the actual Enum. Bear in mind to use the wrapper struct as sql_type on the Enum:

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
2) PGProtocolType: The Wrapper struct that refers to the Postgres Enum type
3) ProtocolType: The Rust Enum that refers to the wrapper struct PGProtocolType

Mixing any of those three types will result in a complicated Diesel trait error. However, these error messages are just
a convoluted way to say that the database type mismatches the Rust type. When you encounter a consulted trait error
message, make sure to check:

1) Do I have a wrapper struct?
2) Does my a wrapper struct derives SqlType?
3) Does my wrapper type has both, sql_type and postgres_type declared?
4) Are sql_type and postgres_type both referring to the correct Postgres type?
5) Does my Rust type refers to the wrapper struct type?

If all those checks pass and you still see errors, it’s most likely a serialization error.

To serialize and deserialize a custom Enum, you write a custom toSql and fromSql implementation. Luckily, this is
straightforward.

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

impl FromSql<PgProtocolType, Pg> for ProtocolType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"UnknownProtocol" => Ok(ProtocolType::UnknownProtocol),
            b"GRPC" => Ok(ProtocolType::GRPC),
            b"HTTP" => Ok(ProtocolType::HTTP),
            b"UDP" => Ok(ProtocolType::UDP),
            _ => Ok(ProtocolType::UnknownProtocol),
        }
    }
}
```

In toSql, it is important to end the implementation with Ok(IsNull::No). In the from_sql, it is important to add a catch
all case that returns an UnknownProtocol instance. In practice, the catch all rarely ever gets triggered, but in those
few corner cases when it does, it keeps the application running.

### Rust Struct Endpoint

```rust 
#[derive(Debug, Clone, FromSqlRow, AsExpression, PartialEq, Eq)]
#[diesel(sql_type=crate::schema::smdb::sql_types::ServiceEndpoint)]
pub struct Endpoint {
    pub name: String,
    pub version: i32,
    pub base_uri: String,
    pub port: i32,
    pub protocol: ProtocolType,
}
```

It is worth mentioning is that the sql_type refers to the wrapper struct generated by Diesel and stored in the schema.rs
file. Keep this in mind if you ever refactor the schema.rs file into a different folder because the macro annotation
checks the path during compilation and throws an error if it cannot find the type in the provided path. The serialize
and deserialize implementation of a custom type is not as obvious as the Enum because, internally, Postgres represent a
custom type as an anonymous typed tuple. Therefore, you have to map a struct to a typed tuple and back.

Let’s start with the toSql implementation to store the Rust Endpoint struct as typed tuple. Luckily, Diesel provides a
helper util to do just that.

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

I cannot stress enough that it is paramount that the tuple type signature must match exactly the Postgres type signature defined in your up.sql. Ideally, you want to use the split view function in your IDE to have the up.sql 
in one pane and the the toSql implementation in another pane, both side by side, to double check 
that the number and types match. 
If the type or number of types mismatch, you will get a compiler error telling you that somehow either 
the number of fields don’t match or that the type of the fields don’t match. 
Also, because the write_tuple expects values, you have to call either to_owned() or clone() on any referenced data.

The from_sql reverses the process by converting a Postgres typed tuple back into a Rust struct. 
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

Similar to the serialization process, it is paramount that the type annotation match exactly the one used in toSql and
the type definition in up.sql.

Debugging serialization issues in Diesel is relatively straight forward since the compiler usually points out the
location of the issue and, often, the issue is a type mismatch that is relatively easy to fix.

It’s worth repeating that you are dealing with three types in total:

1) service_endpoint: The Postgres custom type
2) ServiceEndpoint: The wrapper struct generated by Diesel that refers to the Postgres type service_endpoint. Also, only
   the wrapper struct carries the postgres_type annotation in addition to the an sql_type annotation.
3) Endpoint: The Rust struct with an sql_type annotation referring to the wrapper struct ServiceEndpoint

Make sure all of those types match correctly.

### Rust Struct Service

The service struct gets its serialization and deserialization implementation generated by Diesel so that saves some
typing.
On the other hand, it is a good practice to implement database operations on the actual type itself.
The wisdom here is twofold. For once, you cannot separate the database operation from the type,
therefore it makes sense to implement the operations on the type itself. This also hides implementation details
in encapsulation,
as [discussed in the book](https://doc.rust-lang.org/book/ch17-01-what-is-oo.html#encapsulation-that-hides-implementation-details).

Second, you gain a single point of maintenance because, in case your table definition changes, you have to update the
Rust type anyways and because Diesel is all statically checked, the compiler will immediately point out where your
operations need to be corrected to match the new type definition.

The requirements stated we want CRUD operations, and type-based programming suggest creating a different type per
operation. Because we use external primary keys, we don’t need an additional type for the delete operation, therefore,
in total we crate 3 service types:

1) Service: For Read
2) CreateService: For create operations
3) UpdateService: For update operations

The Service and CreateService have identical fields. Also, both types require a primary key annotation in addition to
the table name. The UpdateServce, however, has each field wrapped into an option type. When the option type is Some,
Diesel knows that this field needs updating. If the field is set to None, Diesel ignores it.

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

###  **Read**

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

“Unmentioned in the requirements, a service depends on other services, and it would be great to test, before deploying a
new service, if all of its dependencies are online.”

Here, we have to implement a method to set a service online and another one to set it offline. By experience, a few
non-trivial errors are caused by incorrectly set Boolean flags and whenever you can, hide Boolean flags in your public
API and use a dedicated method instead to the set them.

Next, we need a method to return true if a service is online. At this point, we also want to add a method that checks if
a service actually is in the database. Imagine, you want to test if you can deploy a new service with, say 5
dependencies, and somehow the database returns no, you can’t. The obvious question is why? There might be some services
that are offline, fair enough, just start them, but it might also be possible that somehow a service isn’t currently in
the database and that is where you need another check method.

Then, we want another method that takes a collection of service ID’s, checks all of them and returns either true if all
services are online, or returns an error message that states which service is offline.

And because we are already here, let’s add another method that returns all services that are online and another one that
returns all services that are currently offline. The latter is handy to quickly identify any deployment blocker.

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

At this point, the Service type implementation is complete. I have skipped the testing procedure, but in a nutshell, you
just create a util that connects to a running Postgres server and then returns a connection to run all tests. You find
all tests in the test folder.

## Applied Best Practices

Implementing the database operations on the Service type is simple, straight forward,
and follows three best practices:

1) Take the database connection as a parameter
2) Prefer the generated DSL over custom queries whenever possible
3) Add type annotations on database return types

### Database connection as a parameter

The first one refers to the idea that no type should hold application state, and a database connection definitely counts as application state. Instead, you would write a database connection manager 
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

Run the compile and see that it works. Now, if you were to remove the type annotation <i64> from get_result, you get a
compile error saying that trait FromSql is not implemented.

If you write a database layer for an existing system, this technique comes in handy as you can seamlessly convert
between Diesel DB types and your target types while leaving your target types as it. And because you never know when you have to do this, it’s generally recommended to add type annotations to DB return types. 

