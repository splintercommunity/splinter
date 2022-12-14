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

//! This module defines the REST API endpoints for interacting with registries.

mod error;
mod nodes;
mod nodes_identity;
mod resources;

use splinter::rest_api::actix_web_1::{Resource, RestResourceProvider};
#[cfg(feature = "authorization")]
use splinter::rest_api::auth::authorization::Permission;

use splinter::registry::RwRegistry;

#[cfg(feature = "authorization")]
const REGISTRY_READ_PERMISSION: Permission = Permission::Check {
    permission_id: "registry.read",
    permission_display_name: "Registry read",
    permission_description: "Allows the client to read the registry",
};
#[cfg(feature = "authorization")]
const REGISTRY_WRITE_PERMISSION: Permission = Permission::Check {
    permission_id: "registry.write",
    permission_display_name: "Registry write",
    permission_description: "Allows the client to modify the registry",
};

pub struct RwRegistryRestResourceProvider {
    resources: Vec<Resource>,
}

impl RwRegistryRestResourceProvider {
    pub fn new(registry: &dyn RwRegistry) -> Self {
        let resources = vec![
            nodes_identity::make_nodes_identity_resource(registry.clone_box()),
            nodes::make_nodes_resource(registry.clone_box()),
        ];
        Self { resources }
    }
}

/// The `RwRegistryRestResourceProvider` struct provides the following endpoints
/// as REST API resources:
///
/// * `GET /registry/nodes` - List the nodes in the registry
/// * `POST /registry/nodes` - Add a node to the registry
/// * `GET /registry/nodes/{identity}` - Fetch a specific node in the registry
/// * `PUT /registry/nodes/{identity}` - Replace a node in the registry
/// * `DELETE /registry/nodes/{identity}` - Delete a node from the registry
impl RestResourceProvider for RwRegistryRestResourceProvider {
    fn resources(&self) -> Vec<Resource> {
        self.resources.clone()
    }
}
