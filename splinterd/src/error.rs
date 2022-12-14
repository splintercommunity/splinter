// Copyright 2018-2022 Cargill Incorporated
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::error::Error;
use std::fmt;
use std::io;

use splinter::error::InternalError;
use splinter::transport::socket::TlsInitError;

use crate::config::ConfigError;
use crate::daemon::StartError;

#[derive(Debug)]
pub enum UserError {
    TransportError(GetTransportError),
    #[cfg(feature = "tap")]
    MissingArgument(String),
    InvalidArgument(String),
    ConfigError(ConfigError),
    IoError {
        context: String,
        source: Option<Box<io::Error>>,
    },
    DaemonError {
        context: String,
        source: Option<Box<dyn Error>>,
    },
    InternalError(InternalError),
}

impl UserError {
    pub fn io_err_with_source(context: &str, err: Box<io::Error>) -> Self {
        UserError::IoError {
            context: context.into(),
            source: Some(err),
        }
    }

    pub fn daemon_err_with_source(context: &str, err: Box<dyn Error>) -> Self {
        UserError::DaemonError {
            context: context.into(),
            source: Some(err),
        }
    }
}

impl Error for UserError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            UserError::TransportError(err) => Some(err),
            #[cfg(feature = "tap")]
            UserError::MissingArgument(_) => None,
            UserError::InvalidArgument(_) => None,
            UserError::ConfigError(err) => Some(err),
            UserError::IoError { source, .. } => source.as_ref().map(|err| &**err as &dyn Error),
            UserError::DaemonError { source, .. } => source.as_ref().map(|err| &**err),
            UserError::InternalError(err) => Some(err),
        }
    }
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UserError::TransportError(err) => write!(f, "unable to get transport: {}", err),
            #[cfg(feature = "tap")]
            UserError::MissingArgument(msg) => write!(f, "missing required argument: {}", msg),
            UserError::InvalidArgument(msg) => write!(f, "required argument is invalid: {}", msg),
            UserError::ConfigError(msg) => {
                write!(f, "error occurred building config object: {}", msg)
            }
            UserError::IoError { context, source } => {
                if let Some(ref err) = source {
                    write!(f, "{}: {}", context, err)
                } else {
                    f.write_str(context)
                }
            }
            UserError::DaemonError { context, source } => {
                if let Some(ref err) = source {
                    write!(f, "{}: {}", context, err)
                } else {
                    f.write_str(context)
                }
            }
            UserError::InternalError(err) => write!(f, "{}", err),
        }
    }
}

impl From<io::Error> for UserError {
    fn from(io_error: io::Error) -> Self {
        UserError::io_err_with_source("encountered IO error", Box::new(io_error))
    }
}

impl From<StartError> for UserError {
    fn from(error: StartError) -> Self {
        UserError::daemon_err_with_source("unable to start the Splinter daemon", Box::new(error))
    }
}

impl From<GetTransportError> for UserError {
    fn from(error: GetTransportError) -> Self {
        UserError::TransportError(error)
    }
}

impl From<ConfigError> for UserError {
    fn from(error: ConfigError) -> Self {
        UserError::ConfigError(error)
    }
}

impl From<InternalError> for UserError {
    fn from(error: InternalError) -> Self {
        UserError::InternalError(error)
    }
}

#[derive(Debug)]
pub enum GetTransportError {
    Cert(String),
    TlsTransport(TlsInitError),
    Io(io::Error),
}

impl Error for GetTransportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            GetTransportError::Cert(_) => None,
            GetTransportError::TlsTransport(err) => Some(err),
            GetTransportError::Io(err) => Some(err),
        }
    }
}

impl fmt::Display for GetTransportError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GetTransportError::Cert(msg) => {
                write!(f, "unable to retrieve certificate: {}", msg)
            }
            GetTransportError::TlsTransport(err) => {
                write!(f, "unable to create TLS transport: {}", err)
            }
            GetTransportError::Io(err) => {
                write!(f, "unable to get transport due to IoError: {}", err)
            }
        }
    }
}

impl From<TlsInitError> for GetTransportError {
    fn from(tls_error: TlsInitError) -> Self {
        GetTransportError::TlsTransport(tls_error)
    }
}

impl From<io::Error> for GetTransportError {
    fn from(io_error: io::Error) -> Self {
        GetTransportError::Io(io_error)
    }
}
