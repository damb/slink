use std::fmt;
use std::str;

use crate::ProtocolErrorV4;

/// Authentication method types.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AuthMethod {
    /// User-Password method.
    UserPass(String, String),
    /// JSON Web Token (RFC 7519).
    JWT(String),
}

impl str::FromStr for AuthMethod {
    type Err = ProtocolErrorV4;

    fn from_str(s: &str) -> Result<AuthMethod, Self::Err> {
        let split: Vec<&str> = s.split(' ').collect();
        if split.is_empty() {
            return Err(ProtocolErrorV4::incorrect_arguments());
        }

        Ok(match split[0].to_lowercase().as_str() {
            "userpass" => {
                let credentials = &split[1..];
                if credentials.len() != 2 {
                    return Err(ProtocolErrorV4::incorrect_arguments());
                }
                Self::UserPass(credentials[0].into(), credentials[1].into())
            }
            "token" => {
                let credentials = &split[1..];
                if credentials.len() != 1 {
                    return Err(ProtocolErrorV4::incorrect_arguments());
                }
                Self::JWT(credentials[0].into())
            }
            other => {
                return Err(ProtocolErrorV4::incorrect_arguments());
            }
        })
    }
}

/// Representation of the SeedLink `AUTH` command.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Auth {
    method: AuthMethod,
}

impl Auth {
    pub const NAME: &'static str = "auth";

    /// Creates a new `AUTH` command.
    pub fn new(method: AuthMethod) -> Self {
        Self { method }
    }
}

impl str::FromStr for Auth {
    type Err = ProtocolErrorV4;
    fn from_str(s: &str) -> Result<Auth, Self::Err> {
        Ok(Self {
            method: AuthMethod::from_str(s)?,
        })
    }
}

impl fmt::Display for Auth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self.method {
            AuthMethod::UserPass(ref user, ref pass) => {
                format!("userpass {} {}", user, pass)
            }
            AuthMethod::JWT(ref token) => {
                format!("jwt {}", token)
            }
        };
        write!(f, "{} {}", Self::NAME, s)
    }
}
