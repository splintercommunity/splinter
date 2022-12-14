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

use super::RefreshTokenStoreOperations;
use crate::biome::refresh_tokens::store::{
    diesel::{models::NewRefreshToken, schema::refresh_tokens},
    RefreshTokenError,
};
use diesel::{dsl::insert_into, prelude::*};

pub(in crate::biome) trait RefreshTokenStoreAddTokenOperation {
    fn add_token(&self, user_id: &str, token: &str) -> Result<(), RefreshTokenError>;
}

#[cfg(feature = "postgres")]
impl<'a> RefreshTokenStoreAddTokenOperation
    for RefreshTokenStoreOperations<'a, diesel::pg::PgConnection>
{
    fn add_token(&self, user_id: &str, token: &str) -> Result<(), RefreshTokenError> {
        insert_into(refresh_tokens::table)
            .values(NewRefreshToken { user_id, token })
            .execute(self.conn)
            .map_err(|err| RefreshTokenError::OperationError {
                context: "Failed to create token".to_string(),
                source: Box::new(err),
            })?;
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl<'a> RefreshTokenStoreAddTokenOperation
    for RefreshTokenStoreOperations<'a, diesel::sqlite::SqliteConnection>
{
    fn add_token(&self, user_id: &str, token: &str) -> Result<(), RefreshTokenError> {
        insert_into(refresh_tokens::table)
            .values(NewRefreshToken { user_id, token })
            .execute(self.conn)
            .map_err(|err| RefreshTokenError::OperationError {
                context: "Failed to create token".to_string(),
                source: Box::new(err),
            })?;
        Ok(())
    }
}
