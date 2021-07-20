// Copyright 2018-2021 Cargill Incorporated
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

//! Provides sql migration scripts and methods for executing
//! migrations.
//!
//! ```ignore
//! use migrations::run_postgres_migrations;
//! use diesel::{pg::PgConnection};
//!
//! let connection = PgConnection::establish(
//!      "postgres://admin:admin@localhost:5432/splinterd").unwrap();
//!
//! run_postgres_migrations(&connection).unwrap();
//!
//! ```

#[cfg(feature = "diesel")]
mod diesel;

#[cfg(feature = "postgres")]
pub use self::diesel::postgres::any_pending_migrations as any_pending_postgres_migrations;
#[cfg(feature = "postgres")]
pub use self::diesel::postgres::run_migrations as run_postgres_migrations;
#[cfg(feature = "sqlite")]
pub use self::diesel::sqlite::any_pending_migrations as any_pending_sqlite_migrations;
#[cfg(feature = "sqlite")]
pub use self::diesel::sqlite::run_migrations as run_sqlite_migrations;
