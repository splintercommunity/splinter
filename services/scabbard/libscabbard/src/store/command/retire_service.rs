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

use std::sync::Arc;

use splinter::{
    error::InternalError, service::FullyQualifiedServiceId, store::command::StoreCommand,
};

use crate::store::{ScabbardStoreFactory, ServiceStatus};

pub struct ScabbardRetireServiceCommand<C> {
    store_factory: Arc<dyn ScabbardStoreFactory<C>>,
    service_id: FullyQualifiedServiceId,
}

impl<C> ScabbardRetireServiceCommand<C> {
    pub fn new(
        store_factory: Arc<dyn ScabbardStoreFactory<C>>,
        service_id: FullyQualifiedServiceId,
    ) -> Self {
        Self {
            store_factory,
            service_id,
        }
    }
}

impl<C> StoreCommand for ScabbardRetireServiceCommand<C> {
    type Context = C;

    fn execute(&self, conn: &Self::Context) -> Result<(), InternalError> {
        let store = self.store_factory.new_store(conn);

        let service = store
            .get_service(&self.service_id)
            .map_err(|err| InternalError::from_source(Box::new(err)))?
            .ok_or_else(|| {
                InternalError::with_message(format!("Unable to fetch service {}", self.service_id))
            })?
            .into_builder()
            .with_status(&ServiceStatus::Retired)
            .build()
            .map_err(|err| InternalError::from_source(Box::new(err)))?;

        store
            .update_service(service)
            .map_err(|err| InternalError::from_source(Box::new(err)))?;

        Ok(())
    }
}
