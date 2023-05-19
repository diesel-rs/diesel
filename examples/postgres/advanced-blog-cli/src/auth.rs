use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use diesel::prelude::*;
use diesel::{self, insert_into};

use crate::schema::users;

const SALT_STRING: &str = "dec7901d02ee422ba6bd0333e4fef137";

#[derive(Debug)]
pub enum AuthenticationError {
    IncorrectPassword,
    NoUsernameSet,
    NoPasswordSet,
    EnvironmentError(dotenvy::Error),
    Argon2Error(argon2::password_hash::Error),
    DatabaseError(diesel::result::Error),
}

impl From<argon2::password_hash::Error> for AuthenticationError {
    fn from(e: argon2::password_hash::Error) -> Self {
        AuthenticationError::Argon2Error(e)
    }
}

pub use self::AuthenticationError::{IncorrectPassword, NoPasswordSet, NoUsernameSet};

#[derive(Queryable, Identifiable, Debug, PartialEq, Eq)]
pub struct User {
    pub id: i32,
    pub username: String,
}

#[derive(Queryable)]
pub struct UserWithPassword {
    user: User,
    password: String,
}

pub fn current_user_from_env(conn: &mut PgConnection) -> Result<Option<User>, AuthenticationError> {
    let username = get_username()?;
    let password = get_password()?;
    find_user(conn, &username, &password)
}

pub fn register_user_from_env(conn: &mut PgConnection) -> Result<User, AuthenticationError> {
    let username = get_username()?;
    let password = get_password()?;
    register_user(conn, &username, &password)
}

fn find_user(
    conn: &mut PgConnection,
    username: &str,
    password: &str,
) -> Result<Option<User>, AuthenticationError> {
    let user_and_password = users::table
        .filter(users::username.eq(username))
        .select(((users::id, users::username), users::hashed_password))
        .first::<UserWithPassword>(conn)
        .optional()
        .map_err(AuthenticationError::DatabaseError)?;

    if let Some(user_and_password) = user_and_password {
        let parsed_hash = PasswordHash::new(&user_and_password.password)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|e| match e {
                argon2::password_hash::Error::Password => IncorrectPassword,
                _ => AuthenticationError::Argon2Error(e),
            })?;
        Ok(Some(user_and_password.user))
    } else {
        Ok(None)
    }
}

fn register_user(
    conn: &mut PgConnection,
    username: &str,
    password: &str,
) -> Result<User, AuthenticationError> {
    // In real applications you should never use a constant salt!!
    // Checkout https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html#salting for details
    let salt = SaltString::from_b64(SALT_STRING)?;
    let argon2 = Argon2::default();
    let hashed_password = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    insert_into(users::table)
        .values((
            users::username.eq(username),
            users::hashed_password.eq(hashed_password),
        ))
        .returning((users::id, users::username))
        .get_result(conn)
        .map_err(AuthenticationError::DatabaseError)
}

fn get_username() -> Result<String, AuthenticationError> {
    if_not_present(dotenvy::var("BLOG_USERNAME"), NoUsernameSet)
}

fn get_password() -> Result<String, AuthenticationError> {
    if_not_present(dotenvy::var("BLOG_PASSWORD"), NoPasswordSet)
}

fn if_not_present<T>(
    res: Result<T, dotenvy::Error>,
    on_not_present: AuthenticationError,
) -> Result<T, AuthenticationError> {
    use std::env::VarError::NotPresent;

    res.map_err(|e| match e {
        dotenvy::Error::EnvVar(NotPresent) => on_not_present,
        e => AuthenticationError::EnvironmentError(e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    use assert_matches::assert_matches;
    use std::env;

    #[test]
    fn current_user_from_env_fails_when_no_username_set() {
        let _guard = this_test_modifies_env();
        env::remove_var("BLOG_USERNAME");

        let conn = &mut connection();

        assert_matches!(current_user_from_env(conn), Err(NoUsernameSet));
    }

    #[test]
    fn current_user_from_env_fails_when_no_password_set() {
        let _guard = this_test_modifies_env();
        env::remove_var("BLOG_PASSWORD");
        env::set_var("BLOG_USERNAME", "sgrif");

        let conn = &mut connection();

        assert_matches!(current_user_from_env(conn), Err(NoPasswordSet));
    }

    #[test]
    fn current_user_returns_none_when_no_user_exists_with_username() {
        let conn = &mut connection();

        assert_matches!(find_user(conn, "sgrif", "hunter2"), Ok(None));
    }

    #[test]
    fn current_user_returns_the_user_if_it_has_the_same_password() {
        let conn = &mut connection();

        let expected_user = register_user(conn, "sgrif", "hunter2").unwrap();
        let user = find_user(conn, "sgrif", "hunter2").unwrap();

        assert_eq!(Some(expected_user), user);
    }

    #[test]
    fn current_user_fails_if_password_does_not_match() {
        let conn = &mut connection();

        register_user(conn, "sgrif", "letmein").unwrap();
        let result = find_user(conn, "sgrif", "hunter2");

        assert_matches!(result, Err(IncorrectPassword));
    }
}
