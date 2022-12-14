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
    diesel::{models::RefreshToken, schema::refresh_tokens},
    RefreshTokenError,
};
use diesel::{prelude::*, result::Error::NotFound};

pub(in crate::biome) trait RefreshTokenStoreFetchTokenOperation {
    fn fetch_token(&self, user_id: &str) -> Result<String, RefreshTokenError>;
}

impl<'a, C> RefreshTokenStoreFetchTokenOperation for RefreshTokenStoreOperations<'a, C>
where
    C: diesel::Connection,
    i64: diesel::deserialize::FromSql<diesel::sql_types::BigInt, C::Backend>,
    String: diesel::deserialize::FromSql<diesel::sql_types::Text, C::Backend>,
{
    fn fetch_token(&self, user_id: &str) -> Result<String, RefreshTokenError> {
        refresh_tokens::table
            .select(refresh_tokens::all_columns)
            .filter(refresh_tokens::user_id.eq(user_id))
            .first::<RefreshToken>(self.conn)
            .map(|t| t.token)
            .map_err(|err| {
                if err == NotFound {
                    RefreshTokenError::NotFoundError(format!(
                        "No refresh token for user {} found",
                        user_id
                    ))
                } else {
                    RefreshTokenError::OperationError {
                        context: format!("Failed to retrieve refresh token for user {}", user_id),
                        source: Box::new(err),
                    }
                }
            })
    }
}
