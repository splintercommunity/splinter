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

#[cfg(feature = "diesel")]
use splinter::error::ConstraintViolationType;
use splinter::error::{
    ConstraintViolationError, InternalError, ResourceTemporarilyUnavailableError,
};

/// Represents ScabbardStore errors
#[derive(Debug)]
pub enum ScabbardStoreError {
    /// Represents errors internal to the function.
    Internal(InternalError),
    /// Represents constraint violations on the database's definition
    ConstraintViolation(ConstraintViolationError),
    /// Represents when the underlying resource is unavailable
    ResourceTemporarilyUnavailable(ResourceTemporarilyUnavailableError),
}

impl Error for ScabbardStoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ScabbardStoreError::Internal(err) => Some(err),
            ScabbardStoreError::ConstraintViolation(err) => Some(err),
            ScabbardStoreError::ResourceTemporarilyUnavailable(err) => Some(err),
        }
    }
}

impl fmt::Display for ScabbardStoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ScabbardStoreError::Internal(err) => write!(f, "{}", err),
            ScabbardStoreError::ConstraintViolation(err) => write!(f, "{}", err),
            ScabbardStoreError::ResourceTemporarilyUnavailable(err) => {
                write!(f, "{}", err)
            }
        }
    }
}

#[cfg(feature = "diesel")]
impl From<diesel::r2d2::PoolError> for ScabbardStoreError {
    fn from(err: diesel::r2d2::PoolError) -> Self {
        ScabbardStoreError::ResourceTemporarilyUnavailable(
            ResourceTemporarilyUnavailableError::from_source(Box::new(err)),
        )
    }
}

#[cfg(feature = "diesel")]
impl From<diesel::result::Error> for ScabbardStoreError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::DatabaseError(db_err_kind, _) => match db_err_kind {
                diesel::result::DatabaseErrorKind::UniqueViolation => {
                    ScabbardStoreError::ConstraintViolation(
                        ConstraintViolationError::from_source_with_violation_type(
                            ConstraintViolationType::Unique,
                            Box::new(err),
                        ),
                    )
                }
                diesel::result::DatabaseErrorKind::ForeignKeyViolation => {
                    ScabbardStoreError::ConstraintViolation(
                        ConstraintViolationError::from_source_with_violation_type(
                            ConstraintViolationType::ForeignKey,
                            Box::new(err),
                        ),
                    )
                }
                _ => ScabbardStoreError::Internal(InternalError::from_source(Box::new(err))),
            },
            _ => ScabbardStoreError::Internal(InternalError::from_source(Box::new(err))),
        }
    }
}

impl From<InternalError> for ScabbardStoreError {
    fn from(err: InternalError) -> Self {
        Self::Internal(err)
    }
}
