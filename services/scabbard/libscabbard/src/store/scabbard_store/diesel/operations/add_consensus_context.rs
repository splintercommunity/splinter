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

use std::convert::TryFrom;

#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;
#[cfg(feature = "sqlite")]
use diesel::sqlite::SqliteConnection;
use diesel::{dsl::insert_into, prelude::*};
use splinter::service::FullyQualifiedServiceId;

use crate::store::scabbard_store::diesel::{
    models::{Consensus2pcContextModel, ContextParticipantList},
    schema::{consensus_2pc_context, consensus_2pc_context_participant},
};
use crate::store::scabbard_store::ConsensusContext;
use crate::store::scabbard_store::ScabbardStoreError;

use super::ScabbardStoreOperations;

pub(in crate::store::scabbard_store::diesel) trait AddContextOperation {
    fn add_consensus_context(
        &self,
        service_id: &FullyQualifiedServiceId,
        context: ConsensusContext,
    ) -> Result<(), ScabbardStoreError>;
}

#[cfg(feature = "sqlite")]
impl<'a> AddContextOperation for ScabbardStoreOperations<'a, SqliteConnection> {
    fn add_consensus_context(
        &self,
        service_id: &FullyQualifiedServiceId,
        context: ConsensusContext,
    ) -> Result<(), ScabbardStoreError> {
        self.conn.transaction::<_, _, _>(|| {
            match context {
                ConsensusContext::TwoPhaseCommit(context) => {
                    let new_context = Consensus2pcContextModel::try_from((&context, service_id))?;
                    let participants =
                        ContextParticipantList::try_from((&context, service_id))?.inner;

                    insert_into(consensus_2pc_context::table)
                        .values(vec![new_context])
                        .execute(self.conn)?;

                    insert_into(consensus_2pc_context_participant::table)
                        .values(participants)
                        .execute(self.conn)?;
                }
            }
            Ok(())
        })
    }
}

#[cfg(feature = "postgres")]
impl<'a> AddContextOperation for ScabbardStoreOperations<'a, PgConnection> {
    fn add_consensus_context(
        &self,
        service_id: &FullyQualifiedServiceId,
        context: ConsensusContext,
    ) -> Result<(), ScabbardStoreError> {
        self.conn.transaction::<_, _, _>(|| {
            match context {
                ConsensusContext::TwoPhaseCommit(context) => {
                    let new_context = Consensus2pcContextModel::try_from((&context, service_id))?;
                    let participants =
                        ContextParticipantList::try_from((&context, service_id))?.inner;

                    insert_into(consensus_2pc_context::table)
                        .values(vec![new_context])
                        .execute(self.conn)?;

                    insert_into(consensus_2pc_context_participant::table)
                        .values(participants)
                        .execute(self.conn)?;
                }
            }
            Ok(())
        })
    }
}