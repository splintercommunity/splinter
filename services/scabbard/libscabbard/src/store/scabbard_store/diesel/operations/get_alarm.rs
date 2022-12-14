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

use std::time::{Duration, SystemTime};

#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;
use diesel::prelude::*;
#[cfg(feature = "sqlite")]
use diesel::sqlite::SqliteConnection;
use splinter::error::{InternalError, InvalidStateError};
use splinter::service::FullyQualifiedServiceId;

use crate::store::scabbard_store::diesel::{
    models::{AlarmTypeModel, ScabbardServiceModel},
    schema::{scabbard_alarm, scabbard_service},
};
use crate::store::scabbard_store::ScabbardStoreError;
use crate::store::AlarmType;

use super::ScabbardStoreOperations;

const OPERATION_NAME: &str = "get_alarm";

pub(in crate::store::scabbard_store::diesel) trait GetAlarmOperation {
    fn get_alarm(
        &self,
        service_id: &FullyQualifiedServiceId,
        alarm_type: &AlarmType,
    ) -> Result<Option<SystemTime>, ScabbardStoreError>;
}

#[cfg(feature = "sqlite")]
impl<'a> GetAlarmOperation for ScabbardStoreOperations<'a, SqliteConnection> {
    fn get_alarm(
        &self,
        service_id: &FullyQualifiedServiceId,
        alarm_type: &AlarmType,
    ) -> Result<Option<SystemTime>, ScabbardStoreError> {
        self.conn.transaction::<_, _, _>(|| {
            // check to see if a service with the given service_id exists
            scabbard_service::table
                .filter(
                    scabbard_service::circuit_id
                        .eq(service_id.circuit_id().to_string())
                        .and(scabbard_service::service_id.eq(service_id.service_id().to_string())),
                )
                .first::<ScabbardServiceModel>(self.conn)
                .optional()
                .map_err(|err| {
                    ScabbardStoreError::from_source_with_operation(err, OPERATION_NAME.to_string())
                })?
                .ok_or_else(|| {
                    ScabbardStoreError::InvalidState(InvalidStateError::with_message(String::from(
                        "Failed to get scabbard alarm, service does not exist",
                    )))
                })?;

            scabbard_alarm::table
                .filter(
                    scabbard_alarm::circuit_id
                        .eq(service_id.circuit_id().to_string())
                        .and(scabbard_alarm::service_id.eq(service_id.service_id().to_string()))
                        .and(scabbard_alarm::alarm_type.eq(AlarmTypeModel::from(alarm_type))),
                )
                .select(scabbard_alarm::alarm)
                .first::<i64>(self.conn)
                .optional()
                .map_err(|err| {
                    ScabbardStoreError::from_source_with_operation(err, OPERATION_NAME.to_string())
                })?
                .map(|t| {
                    SystemTime::UNIX_EPOCH
                        .checked_add(Duration::from_secs(t as u64))
                        .ok_or_else(|| {
                            ScabbardStoreError::Internal(InternalError::with_message(
                                "'alarm' timestamp could not be represented as a `SystemTime`"
                                    .to_string(),
                            ))
                        })
                })
                .transpose()
        })
    }
}

#[cfg(feature = "postgres")]
impl<'a> GetAlarmOperation for ScabbardStoreOperations<'a, PgConnection> {
    fn get_alarm(
        &self,
        service_id: &FullyQualifiedServiceId,
        alarm_type: &AlarmType,
    ) -> Result<Option<SystemTime>, ScabbardStoreError> {
        self.conn.transaction::<_, _, _>(|| {
            // check to see if a service with the given service_id exists
            scabbard_service::table
                .filter(
                    scabbard_service::circuit_id
                        .eq(service_id.circuit_id().to_string())
                        .and(scabbard_service::service_id.eq(service_id.service_id().to_string())),
                )
                .first::<ScabbardServiceModel>(self.conn)
                .optional()
                .map_err(|err| {
                    ScabbardStoreError::from_source_with_operation(err, OPERATION_NAME.to_string())
                })?
                .ok_or_else(|| {
                    ScabbardStoreError::InvalidState(InvalidStateError::with_message(String::from(
                        "Failed to get scabbard alarm, service does not exist",
                    )))
                })?;

            scabbard_alarm::table
                .filter(
                    scabbard_alarm::circuit_id
                        .eq(service_id.circuit_id().to_string())
                        .and(scabbard_alarm::service_id.eq(service_id.service_id().to_string()))
                        .and(scabbard_alarm::alarm_type.eq(AlarmTypeModel::from(alarm_type))),
                )
                .select(scabbard_alarm::alarm)
                .first::<i64>(self.conn)
                .optional()
                .map_err(|err| {
                    ScabbardStoreError::from_source_with_operation(err, OPERATION_NAME.to_string())
                })?
                .map(|t| {
                    SystemTime::UNIX_EPOCH
                        .checked_add(Duration::from_secs(t as u64))
                        .ok_or_else(|| {
                            ScabbardStoreError::Internal(InternalError::with_message(
                                "'alarm' timestamp could not be represented as a `SystemTime`"
                                    .to_string(),
                            ))
                        })
                })
                .transpose()
        })
    }
}
