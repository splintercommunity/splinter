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

//! Module containing ConstraintViolationError implementation.

use std::error;
use std::fmt;

/// The type of constraint violation that caused the error
///
/// This is the type of constraint on the database's definition that is
/// violated. For example, if an operation tries to insert an entry that would
/// cause a duplicate in a unique column, the ConstraintViolationType::Unique
/// should be used.
#[derive(Debug, PartialEq)]
pub enum ConstraintViolationType {
    Unique,
    ForeignKey,
    NotFound,
    Other(String),
}

impl fmt::Display for ConstraintViolationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            ConstraintViolationType::Unique => write!(f, "Unique"),
            ConstraintViolationType::ForeignKey => write!(f, "Foreign Key"),
            ConstraintViolationType::NotFound => f.write_str("Not Found"),
            ConstraintViolationType::Other(ref msg) => write!(f, "{}", msg),
        }
    }
}

/// An error which is returned because of a database constraint violation.
///
/// This error indicates that an update to a database failed because it would have violated
/// a constraint defined as part of the database's definition. For example, if the database has
/// a table with a unique column, and an attempt to insert an entry which would cause duplication in
/// that column, an error with violation type `ConstraintViolationType::Unique` will occur.
///
/// Although this error maps closely to those generated by relational databases (such as those
/// covered by Diesel), the underlying database does not need to be relational. It could, for
/// example, be a memory or file-backed implementation of a store.
pub struct ConstraintViolationError {
    violation_type: ConstraintViolationType,
    source: Option<Box<dyn error::Error>>,
    store: Option<String>,
    operation: Option<String>,
}

impl ConstraintViolationError {
    /// Constructs a new `ConstraintViolationError` from a specified violation type.
    ///
    /// The implementation of `std::fmt::Display` for this error will use the
    /// standard display of the ConstraintViolationType for its message.
    ///
    /// # Examples
    ///
    /// ```
    /// use splinter::error::{ ConstraintViolationError, ConstraintViolationType };
    ///
    /// let constraint_violation_error = ConstraintViolationError::with_violation_type(
    ///     ConstraintViolationType::Unique
    /// );
    /// assert_eq!(format!("{}", constraint_violation_error), "Unique constraint violated");
    /// ```
    pub fn with_violation_type(violation_type: ConstraintViolationType) -> Self {
        Self {
            violation_type,
            source: None,
            store: None,
            operation: None,
        }
    }

    /// Constructs a new `ConstraintViolationError` from a specified source error and violation type.
    ///
    /// The implementation of `std::fmt::Display` for this error will simply pass through the
    /// display of the source message unmodified.
    ///
    /// # Examples
    ///
    /// ```
    /// use splinter::error::{ ConstraintViolationError, ConstraintViolationType };
    ///
    /// let db_err = std::io::Error::new(std::io::ErrorKind::Other, "db error");
    /// let constraint_violation_error = ConstraintViolationError::from_source_with_violation_type(
    ///     ConstraintViolationType::Unique,
    ///     Box::new(db_err)
    /// );
    /// assert_eq!(format!("{}", constraint_violation_error), "db error");
    /// ```
    pub fn from_source_with_violation_type(
        violation_type: ConstraintViolationType,
        source: Box<dyn error::Error>,
    ) -> Self {
        Self {
            violation_type,
            source: Some(source),
            store: None,
            operation: None,
        }
    }

    /// Constructs a new `ConstraintViolationError` from a specified source error and violation
    /// type with store and operation.
    ///
    /// The implementation of `std::fmt::Display` for this error will  pass through the
    /// display of the source message prefixed with the store and operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use splinter::error::{ ConstraintViolationError, ConstraintViolationType };
    ///
    /// let db_err = std::io::Error::new(std::io::ErrorKind::Other, "db error");
    /// let constraint_violation_error =
    ///     ConstraintViolationError::from_source_with_violation_type_and_store(
    ///          ConstraintViolationType::Unique,
    ///          Box::new(db_err),
    ///         "DbStore".to_string(),
    ///         "db_operation".to_string()
    /// );
    /// assert_eq!(format!("{}", constraint_violation_error), "DbStore db_operation: db error");
    /// ```
    pub fn from_source_with_violation_type_and_store(
        violation_type: ConstraintViolationType,
        source: Box<dyn error::Error>,
        store: String,
        operation: String,
    ) -> Self {
        Self {
            violation_type,
            source: Some(source),
            store: Some(store),
            operation: Some(operation),
        }
    }

    /// Returns the violation type that triggered the error.
    pub fn violation_type(&self) -> &ConstraintViolationType {
        &self.violation_type
    }
}

impl error::Error for ConstraintViolationError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.source.as_ref().map(|s| s.as_ref())
    }
}

impl fmt::Display for ConstraintViolationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut format_string = String::new();
        if let Some(store) = &self.store {
            format_string += &format!("{} ", store);
        }

        if let Some(operation) = &self.operation {
            format_string += &format!("{}: ", operation);
        }

        if let Some(source) = &self.source {
            format_string += &source.to_string();
        } else {
            format_string += &format!("{} constraint violated", self.violation_type);
        }

        write!(f, "{}", format_string)
    }
}

impl fmt::Debug for ConstraintViolationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut debug_struct = f.debug_struct("ConstraintViolationError");

        debug_struct.field("violation_type", &self.violation_type);

        if let Some(source) = &self.source {
            debug_struct.field("source", source);
        }

        if let Some(store) = &self.store {
            debug_struct.field("store", store);
        }

        if let Some(operation) = &self.operation {
            debug_struct.field("operation", operation);
        }

        debug_struct.finish()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Tests that errors constructed with `ConstraintViolationError::with_violation_type`
    /// return a debug string of the form
    /// `format!("ConstraintViolationError { violation_type: {:?} }", type)`.
    #[test]
    fn test_debug_with_violation_type() {
        let debug = "ConstraintViolationError { violation_type: Unique }";
        let err = ConstraintViolationError::with_violation_type(ConstraintViolationType::Unique);
        assert_eq!(format!("{:?}", err), debug);
    }

    /// Tests that errors constructed with `ConstraintViolationError::from_source_with_violation_type`
    /// return a debug string of the form
    /// `format!("ConstraintViolationError { source: {:?}, violation_type: {:?} }", source, type)`.
    #[test]
    fn test_debug_from_source_with_violation_type() {
        let debug = "ConstraintViolationError { violation_type: Unique, source: ConstraintViolationError { violation_type: Unique } }";
        let err = ConstraintViolationError::from_source_with_violation_type(
            ConstraintViolationType::Unique,
            Box::new(ConstraintViolationError::with_violation_type(
                ConstraintViolationType::Unique,
            )),
        );
        assert_eq!(format!("{:?}", err), debug);
    }

    /// Tests that error constructed with `ConstraintViolationError::with_violation_type`
    /// return a display string which specifies the which constraint type was violated.
    #[test]
    fn test_display_with_violation_type() {
        let disp = "Unique constraint violated";
        let err = ConstraintViolationError::with_violation_type(ConstraintViolationType::Unique);
        assert_eq!(format!("{}", err), disp);
    }

    /// Tests that error constructed with `ConstraintViolationError::from_source_with_violation_type`
    /// return a display string which specifies the source's display string.
    #[test]
    fn test_display_from_source_with_violation_type() {
        let disp = "Unique constraint violated";
        let err = ConstraintViolationError::from_source_with_violation_type(
            ConstraintViolationType::Unique,
            Box::new(ConstraintViolationError::with_violation_type(
                ConstraintViolationType::Unique,
            )),
        );
        assert_eq!(format!("{}", err), disp);
    }

    /// Tests that error constructed with
    /// `ConstraintViolationError::from_source_with_violation_type_and_store`
    /// return a display string which specifies the source's display string prefixed by store
    /// and operation.
    #[test]
    fn test_display_from_source_with_violation_type_and_store() {
        let disp = "TestStore test_op: Unique constraint violated";
        let err = ConstraintViolationError::from_source_with_violation_type_and_store(
            ConstraintViolationType::Unique,
            Box::new(ConstraintViolationError::with_violation_type(
                ConstraintViolationType::Unique,
            )),
            "TestStore".to_string(),
            "test_op".to_string(),
        );
        assert_eq!(format!("{}", err), disp);
    }
}
