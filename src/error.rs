use std::{
    error, fmt,
    convert::From,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum ErrorKind {
    Other,
    ConfigurationRead,
    ConfigurationParse,
    CtrlCHandler,
    Logging,
    LogFile,
    Serenity,
    DataGet,
    ManagerRead,
    ManagerWrite,
    LoadEmotes,
    ParseWwwBaseUrl,
    IO,
    Serde,
    Reqwest,
}

#[derive(Debug, Clone)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            message: Self::type_to_str(&kind),
            kind,
        }
    }

    pub fn from<E>(kind: ErrorKind, err: E) -> Self
    where E: error::Error {
        let mut message = Self::type_to_str(&kind);
        if !message.is_empty() {
            message = format!("{}: {}", message, err);
        } else {
            message = format!("{}", err);
        }

        Self {
            kind,
            message,
        }
    }

    pub fn custom(message: &str) -> Self {
        Self {
            kind: ErrorKind::Other,
            message: message.into(),
        }
    }

    fn type_to_str(kind: &ErrorKind) -> String {
        match kind {
            ErrorKind::Other | ErrorKind::Serenity | ErrorKind::IO => "",
            ErrorKind::ConfigurationRead => "could not read the configuration",
            ErrorKind::ConfigurationParse => "could not parse the configuration",
            ErrorKind::CtrlCHandler => "could not set the Ctrl-C handler",
            ErrorKind::Logging => "could not setup logging",
            ErrorKind::LogFile => "could not write to log file",
            ErrorKind::DataGet => "could not get shared manager",
            ErrorKind::ManagerRead => "could not get shared manager read lock",
            ErrorKind::ManagerWrite => "could not get shared manager write lock",
            ErrorKind::LoadEmotes => "could not load emotes from disk",
            ErrorKind::ParseWwwBaseUrl => "could not parse www config base url",
            ErrorKind::Serde => "could not serialize/deserialize JSON",
            ErrorKind::Reqwest => "reqwest error",
        }.into()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}

// This is important for other errors to wrap this one.
impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::from(ErrorKind::IO, err)
    }
}

impl From<serenity::Error> for Error {
    fn from(err: serenity::Error) -> Self {
        Self::from(ErrorKind::Serenity, err)
    }
}

impl From<ctrlc::Error> for Error {
    fn from(err: ctrlc::Error) -> Self {
        Self::from(ErrorKind::CtrlCHandler, err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::from(ErrorKind::Serde, err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::from(ErrorKind::Reqwest, err)
    }
}
