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

use super::CredentialsStoreOperations;
use crate::biome::credentials::store::diesel::schema::user_credentials;
use crate::biome::credentials::store::error::CredentialsStoreError;
use crate::biome::credentials::store::{
    CredentialsBuilder, CredentialsModel, PasswordEncryptionCost,
};
use diesel::{dsl::update, prelude::*, result::Error::NotFound};

pub(in crate::biome::credentials) trait CredentialsStoreUpdateCredentialsOperation {
    fn update_credentials(
        &self,
        user_id: &str,
        username: &str,
        password: &str,
        password_encryption_cost: PasswordEncryptionCost,
    ) -> Result<(), CredentialsStoreError>;
}

impl<'a, C> CredentialsStoreUpdateCredentialsOperation for CredentialsStoreOperations<'a, C>
where
    C: diesel::Connection,
    i64: diesel::deserialize::FromSql<diesel::sql_types::BigInt, C::Backend>,
    String: diesel::deserialize::FromSql<diesel::sql_types::Text, C::Backend>,
{
    fn update_credentials(
        &self,
        user_id: &str,
        username: &str,
        password: &str,
        password_encryption_cost: PasswordEncryptionCost,
    ) -> Result<(), CredentialsStoreError> {
        let credentials_builder: CredentialsBuilder = Default::default();
        let credentials = credentials_builder
            .with_user_id(user_id)
            .with_username(username)
            .with_password(password)
            .with_password_encryption_cost(password_encryption_cost)
            .build()
            .map_err(|err| CredentialsStoreError::OperationError {
                context: "Failed to build updated credentials".to_string(),
                source: Box::new(err),
            })?;
        let credential_exists = user_credentials::table
            .filter(user_credentials::user_id.eq(&credentials.user_id))
            .first::<CredentialsModel>(self.conn)
            .map(Some)
            .or_else(|err| if err == NotFound { Ok(None) } else { Err(err) })
            .map_err(|err| CredentialsStoreError::QueryError {
                context: "Failed check for existing user id".to_string(),
                source: Box::new(err),
            })?;
        if credential_exists.is_none() {
            return Err(CredentialsStoreError::NotFoundError(format!(
                "Credentials not found for user id: {}",
                &credentials.user_id
            )));
        }
        update(user_credentials::table.filter(user_credentials::user_id.eq(&credentials.user_id)))
            .set((
                user_credentials::user_id.eq(&credentials.user_id),
                user_credentials::username.eq(&credentials.username),
                user_credentials::password.eq(&credentials.password),
            ))
            .execute(self.conn)
            .map(|_| ())
            .map_err(|err| CredentialsStoreError::OperationError {
                context: "Failed to update credentials".to_string(),
                source: Box::new(err),
            })?;
        Ok(())
    }
}
