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

#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;
#[cfg(feature = "sqlite")]
use diesel::sqlite::SqliteConnection;
use diesel::{delete, dsl::insert_into, prelude::*, update};
use splinter::error::InvalidStateError;

use crate::store::scabbard_store::diesel::{
    models::{ScabbardPeerModel, ScabbardServiceModel, ServiceStatusTypeModel},
    schema::{scabbard_peer, scabbard_service},
};
use crate::store::scabbard_store::service::ScabbardService;
use crate::store::scabbard_store::ScabbardStoreError;

use super::ScabbardStoreOperations;

const OPERATION_NAME: &str = "update_service";

pub(in crate::store::scabbard_store::diesel) trait UpdateServiceAction {
    fn update_service(&self, service: ScabbardService) -> Result<(), ScabbardStoreError>;
}

#[cfg(feature = "sqlite")]
impl<'a> UpdateServiceAction for ScabbardStoreOperations<'a, SqliteConnection> {
    fn update_service(&self, service: ScabbardService) -> Result<(), ScabbardStoreError> {
        self.conn.transaction::<_, ScabbardStoreError, _>(|| {
            // check to see if the service exists
            scabbard_service::table
                .filter(
                    scabbard_service::circuit_id
                        .eq(service.service_id().circuit_id().to_string())
                        .and(
                            scabbard_service::service_id
                                .eq(service.service_id().service_id().to_string()),
                        ),
                )
                .first::<ScabbardServiceModel>(self.conn)
                .optional()
                .map_err(|err| {
                    ScabbardStoreError::from_source_with_operation(err, OPERATION_NAME.to_string())
                })?
                .ok_or_else(|| {
                    ScabbardStoreError::InvalidState(InvalidStateError::with_message(String::from(
                        "Failed to update service, service does not exist",
                    )))
                })?;

            update(scabbard_service::table)
                .filter(
                    scabbard_service::circuit_id
                        .eq(service.service_id().circuit_id().to_string())
                        .and(
                            scabbard_service::service_id
                                .eq(service.service_id().service_id().to_string()),
                        ),
                )
                .set(scabbard_service::status.eq(ServiceStatusTypeModel::from(service.status())))
                .execute(self.conn)
                .map_err(|err| {
                    ScabbardStoreError::from_source_with_operation(err, OPERATION_NAME.to_string())
                })?;

            if !service.peers().is_empty() {
                delete(
                    scabbard_peer::table.filter(
                        scabbard_peer::circuit_id
                            .eq(service.service_id().circuit_id().to_string())
                            .and(
                                scabbard_peer::service_id
                                    .eq(service.service_id().service_id().to_string()),
                            ),
                    ),
                )
                .execute(self.conn)
                .map_err(|err| {
                    ScabbardStoreError::from_source_with_operation(err, OPERATION_NAME.to_string())
                })?;

                insert_into(scabbard_peer::table)
                    .values(Vec::<ScabbardPeerModel>::from(&service))
                    .execute(self.conn)
                    .map_err(|err| {
                        ScabbardStoreError::from_source_with_operation(
                            err,
                            OPERATION_NAME.to_string(),
                        )
                    })?;
            }
            Ok(())
        })
    }
}

#[cfg(feature = "postgres")]
impl<'a> UpdateServiceAction for ScabbardStoreOperations<'a, PgConnection> {
    fn update_service(&self, service: ScabbardService) -> Result<(), ScabbardStoreError> {
        self.conn.transaction::<_, _, _>(|| {
            // check to see if the service exists
            scabbard_service::table
                .filter(
                    scabbard_service::circuit_id
                        .eq(service.service_id().circuit_id().to_string())
                        .and(
                            scabbard_service::service_id
                                .eq(service.service_id().service_id().to_string()),
                        ),
                )
                .first::<ScabbardServiceModel>(self.conn)
                .optional()
                .map_err(|err| {
                    ScabbardStoreError::from_source_with_operation(err, OPERATION_NAME.to_string())
                })?
                .ok_or_else(|| {
                    ScabbardStoreError::InvalidState(InvalidStateError::with_message(String::from(
                        "Failed to update service, service does not exist",
                    )))
                })?;

            update(scabbard_service::table)
                .filter(
                    scabbard_service::circuit_id
                        .eq(service.service_id().circuit_id().to_string())
                        .and(
                            scabbard_service::service_id
                                .eq(service.service_id().service_id().to_string()),
                        ),
                )
                .set(scabbard_service::status.eq(ServiceStatusTypeModel::from(service.status())))
                .execute(self.conn)
                .map_err(|err| {
                    ScabbardStoreError::from_source_with_operation(err, OPERATION_NAME.to_string())
                })?;

            if !service.peers().is_empty() {
                delete(
                    scabbard_peer::table.filter(
                        scabbard_peer::circuit_id
                            .eq(service.service_id().circuit_id().to_string())
                            .and(
                                scabbard_peer::service_id
                                    .eq(service.service_id().service_id().to_string()),
                            ),
                    ),
                )
                .execute(self.conn)
                .map_err(|err| {
                    ScabbardStoreError::from_source_with_operation(err, OPERATION_NAME.to_string())
                })?;

                insert_into(scabbard_peer::table)
                    .values(Vec::<ScabbardPeerModel>::from(&service))
                    .execute(self.conn)
                    .map_err(|err| {
                        ScabbardStoreError::from_source_with_operation(
                            err,
                            OPERATION_NAME.to_string(),
                        )
                    })?;
            }
            Ok(())
        })
    }
}
