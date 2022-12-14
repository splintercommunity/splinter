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

use log::info;
use splinter::{
    error::InternalError,
    service::{FullyQualifiedServiceId, MessageHandler, MessageSender, ServiceType, TimerAlarm},
};

use crate::protocol::v3::{
    message::ScabbardMessage,
    two_phase_commit::{
        Abort, Commit, DecisionAck, DecisionRequest, TwoPhaseCommitMessage, VoteRequest,
        VoteResponse,
    },
};
use crate::protos::FromBytes as _;
use crate::store::{ConsensusEvent, ConsensusType, Event, Message, ScabbardStore};

const SCABBARD_SERVICE_TYPE: ServiceType<'static> = ServiceType::new_static("scabbard:v3");

pub struct ScabbardMessageHandler {
    store: Box<dyn ScabbardStore>,
    alarm: Box<dyn TimerAlarm>,
}

impl ScabbardMessageHandler {
    pub fn new(store: Box<dyn ScabbardStore>, alarm: Box<dyn TimerAlarm>) -> Self {
        Self { store, alarm }
    }
}

impl MessageHandler for ScabbardMessageHandler {
    type Message = ScabbardMessage;

    fn handle_message(
        &mut self,
        _sender: &dyn MessageSender<Self::Message>,
        to_service: FullyQualifiedServiceId,
        from_service: FullyQualifiedServiceId,
        message: Self::Message,
    ) -> Result<(), InternalError> {
        info!(
            "handling scabbard message, to: {} from: {}",
            to_service, from_service
        );

        let service = self
            .store
            .get_service(&to_service)
            .map_err(|e| InternalError::from_source(Box::new(e)))?
            .ok_or_else(|| {
                InternalError::with_message(format!(
                    "Unable to handle messages for {}: does not exist",
                    to_service
                ))
            })?;

        match message {
            ScabbardMessage::ConsensusMessage(msg_bytes) => match service.consensus() {
                ConsensusType::TwoPC => {
                    let message = TwoPhaseCommitMessage::from_bytes(&msg_bytes)
                        .map_err(|e| InternalError::from_source(Box::new(e)))?;
                    let (_, from_service) = from_service.deconstruct();
                    self.store
                        .add_consensus_event(
                            &to_service,
                            ConsensusEvent::TwoPhaseCommit(Event::Deliver(
                                from_service,
                                into_store_msg(message),
                            )),
                        )
                        .map_err(|e| InternalError::from_source(Box::new(e)))?;

                    // wake up the timer for there received message
                    self.alarm
                        .wake_up(SCABBARD_SERVICE_TYPE, Some(to_service))?;
                }
            },
        }

        Ok(())
    }
}

fn into_store_msg(msg: TwoPhaseCommitMessage) -> Message {
    match msg {
        TwoPhaseCommitMessage::VoteRequest(VoteRequest { epoch, value }) => {
            Message::VoteRequest(epoch, value)
        }
        TwoPhaseCommitMessage::VoteResponse(VoteResponse { epoch, response }) => {
            Message::VoteResponse(epoch, response)
        }
        TwoPhaseCommitMessage::Commit(Commit { epoch }) => Message::Commit(epoch),
        TwoPhaseCommitMessage::Abort(Abort { epoch }) => Message::Abort(epoch),
        TwoPhaseCommitMessage::DecisionRequest(DecisionRequest { epoch }) => {
            Message::DecisionRequest(epoch)
        }
        TwoPhaseCommitMessage::DecisionAck(DecisionAck { epoch }) => Message::DecisionAck(epoch),
    }
}
