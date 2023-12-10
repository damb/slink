/// Enumeration of possible SeedLink v4 authentication method types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Auth {
    /// User-Password method.
    UserPass(String, String),
    /// JSON Web Token (RFC 7519).
    JWT(String),
}
