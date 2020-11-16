// Copyright 2018-2020 Cargill Incorporated
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

//! OAuth REST API endpoints

#[cfg(feature = "rest-api-actix")]
mod actix;
mod resources;

use crate::error::InternalError;
use crate::rest_api::{Resource, RestResourceProvider};

use super::{OAuthClient, UserTokens};

/// Perform a save operation on a set of UserTokens.
pub trait SaveTokensOperation: Sync + Send {
    /// Execute a save operation on the given UserTokens.
    ///
    /// # Errors
    ///
    /// Returns an InternalError, if the implementation produces an error during the save
    /// operation.
    fn save_tokens(&self, user_tokens: &UserTokens) -> Result<(), InternalError>;

    fn clone_box(&self) -> Box<dyn SaveTokensOperation>;
}

impl Clone for Box<dyn SaveTokensOperation> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A No-Op SaveTokensOperation.
pub struct SaveTokensToNull;

impl SaveTokensOperation for SaveTokensToNull {
    fn save_tokens(&self, _user_tokens: &UserTokens) -> Result<(), InternalError> {
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn SaveTokensOperation> {
        Box::new(SaveTokensToNull)
    }
}

/// Provides the REST API [Resource](../../../rest_api/struct.Resource.html) definitions for OAuth
/// endpoints. The following endpoints are provided:
///
/// * `GET /oauth/login` - Get the URL for requesting authorization from the provider
/// * `GET /oauth/callback` - Receive the authorization code from the provider
///
/// These endpoints are only available if the following REST API backend feature is enabled:
///
/// * `rest-api-actix`
#[derive(Clone)]
pub(crate) struct OAuthResourceProvider {
    client: OAuthClient,
    save_tokens_operation: Box<dyn SaveTokensOperation>,
}

impl OAuthResourceProvider {
    /// Creates a new `OAuthResourceProvider`
    pub fn new(client: OAuthClient, save_tokens_operation: Box<dyn SaveTokensOperation>) -> Self {
        Self {
            client,
            save_tokens_operation,
        }
    }
}

/// `OAuthResourceProvider` provides the following endpoints as REST API resources:
///
/// * `GET /oauth/login` - Get the URL for requesting authorization from the provider
/// * `GET /oauth/callback` - Receive the authorization code from the provider
///
/// These endpoints are only available if the following REST API backend feature is enabled:
///
/// * `rest-api-actix`
impl RestResourceProvider for OAuthResourceProvider {
    fn resources(&self) -> Vec<Resource> {
        // Allowing unused_mut because resources must be mutable if feature `rest-api-actix` is
        // enabled
        #[allow(unused_mut)]
        let mut resources = Vec::new();

        #[cfg(feature = "rest-api-actix")]
        {
            resources.append(&mut vec![
                actix::login::make_login_route(self.client.clone()),
                actix::callback::make_callback_route(
                    self.client.clone(),
                    self.save_tokens_operation.clone(),
                ),
            ]);
        }

        resources
    }
}