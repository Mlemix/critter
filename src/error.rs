use std::fmt;
use std::error;

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    BadMedia,
    NoUserData,
    TooManyRequests,
    ApiError(String),
    Unknown
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Reqwest(ref err) => write!(f, "HTTP request error: {}", err),
            Error::BadMedia => write!(f, "faulty media"),
            Error::NoUserData => write!(f, "No user data found"),
            Error::ApiError(ref str) => write!(f, "Api Error: {}", str),
            Error::Unknown => write!(f, "unknown"),
            Error::TooManyRequests => write!(f, "too many reqs"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::Reqwest(ref err) => Some(err),
            Error::BadMedia => None,
            Error::NoUserData => None,
            Error::ApiError(_) => None,
            Error::Unknown => None,
            Error::TooManyRequests => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Error {
        Error::Reqwest(err)
    }
}