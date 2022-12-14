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

//! A convenient client for interacting with scabbard services on a Splinter node.

use crate::client::error::ScabbardClientError;

use super::ReqwestScabbardClient;

/// Builder for building a [`ScabbardClient`](crate::client::ScabbardClient).
#[derive(Default)]
pub struct ReqwestScabbardClientBuilder {
    url: Option<String>,
    auth: Option<String>,
}

impl ReqwestScabbardClientBuilder {
    /// Creates a new `ScabbardClientBuilder`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the `url` field of the `ScabbardClientBuilder`. The url will be used
    /// as the bind endpoint for the Splinter REST API.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the bind endpoint of the Splinter REST API.
    pub fn with_url(mut self, url: &str) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the `auth` field of the `ScabbardClientBuilder`. The `auth` string will be
    /// submitted to the Splinter REST API in an Authorization header.
    ///
    /// # Arguments
    ///
    /// * `auth` - The authorization string to be submitted to the Splinter REST API.
    pub fn with_auth(mut self, auth: &str) -> Self {
        self.auth = Some(auth.into());
        self
    }

    /// Builds a `ScabbardClient`.
    ///
    /// # Errors
    ///
    /// Returns an error in any of the following cases:
    /// * Returns an error if url is not set
    /// * Returns an error if auth is not set
    pub fn build(self) -> Result<ReqwestScabbardClient, ScabbardClientError> {
        Ok(ReqwestScabbardClient {
            url: self.url.ok_or_else(|| {
                ScabbardClientError::new("Failed to build client, url not provided")
            })?,
            auth: self.auth.ok_or_else(|| {
                ScabbardClientError::new("Failed to build client, jwt authorization not provided")
            })?,
        })
    }
}
