use ::std::error::Error as StdError;
use ::std::ffi::NulError;

use super::DatabaseErrorInformation;

quick_error! {
    #[derive(Debug)]
    /// The generic "things can fail in a myriad of ways" enum. This type is not
    /// indended to be exhaustively matched, and new variants may be added in the
    /// future without a major version bump.
    pub enum Error {
        InvalidCString(err: NulError) {
            from()
            description(err.description())
            display("Invalid C string: {}", err)
        }
        DatabaseError(kind: DatabaseErrorKind, information: Box<DatabaseErrorInformation+Send>) {
            description(kind.description())
            display("Database error ({}): {}", kind, information.message())
        }
        NotFound {
            description("Record not found")
        }
        DeserializationError(err: Box<StdError+Send+Sync>) {
            description(err.description())
            display("Deserialization error: {}", err)
        }
        SerializationError(err: Box<StdError+Send+Sync>) {
            description(err.description())
            display("Serialization error: {}", err)
        }
        // Match against _ instead, more variants may be added in the future
        #[doc(hidden)] __Nonexhaustive
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (&Error::InvalidCString(ref a), &Error::InvalidCString(ref b)) => a == b,
            (&Error::DatabaseError(_, ref a), &Error::DatabaseError(_, ref b)) =>
                a.message() == b.message(),
            (&Error::NotFound, &Error::NotFound) => true,
            _ => false,
        }
    }
}

quick_error! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    /// The kind of database error that occurred. This is not meant to exhaustively
    /// cover all possible errors, but is used to identify errors which are commonly
    /// recovered from programatically. This enum is not intended to be exhaustively
    /// matched, and new variants may be added in the future without a major version
    /// bump.
    pub enum DatabaseErrorKind {
        UniqueViolation {
            description("Unique violation")
        }
        // Match against _ instead, more variants may be added in the future
        #[doc(hidden)] __Unknown
    }
}

quick_error! {
    #[derive(Debug, PartialEq)]
    pub enum ConnectionError {
        InvalidCString(err: NulError) {
            from()
            description(err.description())
            display("{}", err)
        }
        BadConnection(message: String) {
            description("Bad connection error")
            display("Bad connection error: {}", message)
        }
    }
}
