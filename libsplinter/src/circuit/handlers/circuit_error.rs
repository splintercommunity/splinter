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

use crate::circuit::handlers::create_message;
use crate::circuit::routing::{RoutingTableReader, ServiceId};
use crate::network::dispatch::{DispatchError, Handler, MessageContext, MessageSender, PeerId};
use crate::peer::PeerTokenPair;
use crate::protos::circuit::{CircuitError, CircuitMessageType};

// Implements a handler that handles CircuitError messages
pub struct CircuitErrorHandler {
    node_id: String,
    routing_table: Box<dyn RoutingTableReader>,
}

// In most cases the error message will be returned directly back to service, but in the case
// where it is returned back to a different node, this node will do its best effort to
// return it back to the service or node who sent the original message.
impl Handler for CircuitErrorHandler {
    type Source = PeerId;
    type MessageType = CircuitMessageType;
    type Message = CircuitError;

    fn match_type(&self) -> Self::MessageType {
        CircuitMessageType::CIRCUIT_ERROR_MESSAGE
    }

    fn handle(
        &self,
        msg: Self::Message,
        context: &MessageContext<Self::Source, Self::MessageType>,
        sender: &dyn MessageSender<Self::Source>,
    ) -> Result<(), DispatchError> {
        debug!("Handle Circuit Error Message {:?}", msg);
        let circuit_name = msg.get_circuit_name();
        let service_id = msg.get_service_id();
        let unique_id = ServiceId::new(circuit_name.to_string(), service_id.to_string());

        // check if the msg_sender is in the service directory
        let recipient = match self.routing_table.get_service(&unique_id).map_err(|_| {
            DispatchError::HandleError(format!("Unable to get service: {}", unique_id))
        })? {
            Some(service) => {
                let node_id = service.node_id();
                if node_id == self.node_id {
                    // If the service is connected to this node, send the error to the service
                    match service.local_peer_id() {
                        Some(peer_id) => peer_id.clone(),
                        None => {
                            // This should never happen, as a peer id will always
                            // be set on a service that is connected to the local node.
                            warn!("No peer id for service:{} ", service.service_id());
                            return Ok(());
                        }
                    }
                } else {
                    // If the service is connected to another node, send the error to that node
                    let circuit = self
                        .routing_table
                        .get_circuit(circuit_name)
                        .map_err(|err| DispatchError::HandleError(err.to_string()))?
                        .ok_or_else(|| {
                            DispatchError::HandleError(format!(
                                "Circuit does not exist {}, cannot sent error",
                                node_id
                            ))
                        })?;

                    let peer_id = self
                        .routing_table
                        .get_node(service.node_id())
                        .map_err(|err| DispatchError::HandleError(err.to_string()))?
                        .ok_or_else(|| {
                            DispatchError::HandleError(format!(
                                "Node {} not in routing table",
                                node_id
                            ))
                        })?
                        .get_peer_auth_token(circuit.authorization_type())
                        .map_err(|err| DispatchError::HandleError(err.to_string()))?;

                    let local_peer_id = self
                        .routing_table
                        .get_node(&self.node_id)
                        .map_err(|err| DispatchError::HandleError(err.to_string()))?
                        .ok_or_else(|| {
                            DispatchError::HandleError(format!(
                                "Local Node {} not in routing table",
                                node_id
                            ))
                        })?
                        .get_peer_auth_token(circuit.authorization_type())
                        .map_err(|err| DispatchError::HandleError(err.to_string()))?;

                    PeerTokenPair::new(peer_id, local_peer_id)
                }
            }
            None => {
                // If the service is not in the service directory, the nodes does not know who to
                // forward this message to, so the message is dropped
                warn!(
                    "Original message sender is not connected: {}, cannot send Circuit Error",
                    service_id
                );
                return Ok(());
            }
        };

        let network_msg_bytes = create_message(
            context.message_bytes().to_vec(),
            CircuitMessageType::CIRCUIT_ERROR_MESSAGE,
        )?;

        // forward error message
        sender
            .send(recipient.into(), network_msg_bytes)
            .map_err(|(recipient, payload)| {
                DispatchError::NetworkSendError((recipient.into(), payload))
            })?;
        Ok(())
    }
}

impl CircuitErrorHandler {
    pub fn new(node_id: String, routing_table: Box<dyn RoutingTableReader>) -> Self {
        CircuitErrorHandler {
            node_id,
            routing_table,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use protobuf::Message;

    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use crate::circuit::routing::AuthorizationType;
    use crate::circuit::routing::{
        memory::RoutingTable, Circuit, CircuitNode, RoutingTableWriter, Service,
    };
    use crate::network::dispatch::Dispatcher;
    use crate::peer::PeerAuthorizationToken;
    use crate::protos::circuit::{CircuitError_Error, CircuitMessage};
    use crate::protos::network::{NetworkEcho, NetworkMessage, NetworkMessageType};

    // Test that if an error message received is meant for the service connected to a node,
    // the error message is sent to the service
    #[test]
    fn test_circuit_error_handler_service() {
        // Set up dispatcher and mock sender
        let mock_sender = MockSender::new();
        let mut dispatcher = Dispatcher::new(Box::new(mock_sender.clone()));

        let table = RoutingTable::default();
        let reader: Box<dyn RoutingTableReader> = Box::new(table.clone());
        let mut writer: Box<dyn RoutingTableWriter> = Box::new(table.clone());

        let node_123 = CircuitNode::new("123".to_string(), vec!["123.0.0.1:0".to_string()], None);
        let node_345 = CircuitNode::new("345".to_string(), vec!["123.0.0.1:1".to_string()], None);

        let mut service_abc = Service::new(
            "abc".to_string(),
            "test".to_string(),
            "123".to_string(),
            vec![],
        );
        let mut service_def = Service::new(
            "def".to_string(),
            "test".to_string(),
            "345".to_string(),
            vec![],
        );

        // Add circuit and service to splinter state
        let circuit = Circuit::new(
            "alpha".into(),
            vec![service_abc.clone(), service_def.clone()],
            vec!["123".into(), "345".into()],
            AuthorizationType::Trust,
        );

        writer
            .add_circuit(
                circuit.circuit_id().into(),
                circuit,
                vec![node_123, node_345],
            )
            .expect("Unable to add circuits");

        let abc_id = ServiceId::new("alpha".into(), "abc".into());
        let def_id = ServiceId::new("alpha".into(), "def".into());
        service_abc.set_local_peer_id(PeerTokenPair::new(
            PeerAuthorizationToken::from_peer_id("abc_network"),
            PeerAuthorizationToken::from_peer_id("123"),
        ));
        service_def.set_local_peer_id(PeerTokenPair::new(
            PeerAuthorizationToken::from_peer_id("def_network"),
            PeerAuthorizationToken::from_peer_id("345"),
        ));
        writer.add_service(abc_id, service_abc).unwrap();
        writer.add_service(def_id, service_def).unwrap();

        // Add circuit error handler to the the dispatcher
        let handler = CircuitErrorHandler::new("123".to_string(), reader);
        dispatcher.set_handler(Box::new(handler));

        // Create the error message
        let mut circuit_error = CircuitError::new();
        circuit_error.set_service_id("abc".into());
        circuit_error.set_circuit_name("alpha".into());
        circuit_error.set_correlation_id("1234".into());
        circuit_error.set_error(CircuitError_Error::ERROR_RECIPIENT_NOT_IN_DIRECTORY);
        circuit_error.set_error_message("TEST".into());
        let error_bytes = circuit_error.write_to_bytes().unwrap();

        // dispatch the error message
        dispatcher
            .dispatch(
                PeerTokenPair::new(
                    PeerAuthorizationToken::from_peer_id("345"),
                    PeerAuthorizationToken::from_peer_id("123"),
                )
                .into(),
                &CircuitMessageType::CIRCUIT_ERROR_MESSAGE,
                error_bytes.clone(),
            )
            .unwrap();

        let (id, message) = mock_sender.next_outbound().expect("No message was sent");
        assert_network_message(
            message,
            id.into(),
            PeerTokenPair::new(
                PeerAuthorizationToken::from_peer_id("abc_network"),
                PeerAuthorizationToken::from_peer_id("123"),
            ),
            CircuitMessageType::CIRCUIT_ERROR_MESSAGE,
            |msg: CircuitError| {
                assert_eq!(msg.get_service_id(), "abc");
                assert_eq!(msg.get_circuit_name(), "alpha");
                assert_eq!(
                    msg.get_error(),
                    CircuitError_Error::ERROR_RECIPIENT_NOT_IN_DIRECTORY
                );
                assert_eq!(msg.get_error_message(), "TEST");
                assert_eq!(msg.get_correlation_id(), "1234");
            },
        )
    }

    // Test that if an error message received is meant for the service not connected to this node,
    // the error message is sent to the node the service is connected to
    #[test]
    fn test_circuit_error_handler_node() {
        // Set up dispatcher and mock sender
        let mock_sender = MockSender::new();
        let mut dispatcher = Dispatcher::new(Box::new(mock_sender.clone()));

        let table = RoutingTable::default();
        let reader: Box<dyn RoutingTableReader> = Box::new(table.clone());
        let mut writer: Box<dyn RoutingTableWriter> = Box::new(table.clone());

        let node_123 = CircuitNode::new("123".to_string(), vec!["123.0.0.1:0".to_string()], None);
        let node_345 = CircuitNode::new("345".to_string(), vec!["123.0.0.1:1".to_string()], None);

        let mut service_abc = Service::new(
            "abc".to_string(),
            "test".to_string(),
            "123".to_string(),
            vec![],
        );
        let mut service_def = Service::new(
            "def".to_string(),
            "test".to_string(),
            "345".to_string(),
            vec![],
        );

        // Add circuit and service to splinter state
        let circuit = Circuit::new(
            "alpha".into(),
            vec![service_abc.clone(), service_def.clone()],
            vec!["123".into(), "345".into()],
            AuthorizationType::Trust,
        );

        writer
            .add_circuit(
                circuit.circuit_id().into(),
                circuit,
                vec![node_123, node_345],
            )
            .expect("Unable to add circuits");

        let abc_id = ServiceId::new("alpha".into(), "abc".into());
        let def_id = ServiceId::new("alpha".into(), "def".into());
        service_abc.set_local_peer_id(PeerTokenPair::new(
            PeerAuthorizationToken::from_peer_id("abc_network"),
            PeerAuthorizationToken::from_peer_id("123"),
        ));
        service_def.set_local_peer_id(PeerTokenPair::new(
            PeerAuthorizationToken::from_peer_id("def_network"),
            PeerAuthorizationToken::from_peer_id("345"),
        ));

        writer.add_service(abc_id, service_abc).unwrap();
        writer.add_service(def_id, service_def).unwrap();

        // Add circuit error handler to the the dispatcher
        let handler = CircuitErrorHandler::new("123".to_string(), reader);
        dispatcher.set_handler(Box::new(handler));

        // Create the error message
        let mut circuit_error = CircuitError::new();
        circuit_error.set_service_id("def".into());
        circuit_error.set_circuit_name("alpha".into());
        circuit_error.set_correlation_id("1234".into());
        circuit_error.set_error(CircuitError_Error::ERROR_RECIPIENT_NOT_IN_DIRECTORY);
        circuit_error.set_error_message("TEST".into());
        let error_bytes = circuit_error.write_to_bytes().unwrap();

        // dispatch the error message
        dispatcher
            .dispatch(
                PeerTokenPair::new(
                    PeerAuthorizationToken::from_peer_id("568"),
                    PeerAuthorizationToken::from_peer_id("345"),
                )
                .into(),
                &CircuitMessageType::CIRCUIT_ERROR_MESSAGE,
                error_bytes.clone(),
            )
            .unwrap();

        let (id, message) = mock_sender.next_outbound().expect("No message was sent");
        assert_network_message(
            message,
            id.into(),
            PeerTokenPair::new(
                PeerAuthorizationToken::from_peer_id("345"),
                PeerAuthorizationToken::from_peer_id("123"),
            ),
            CircuitMessageType::CIRCUIT_ERROR_MESSAGE,
            |msg: CircuitError| {
                assert_eq!(msg.get_service_id(), "def");
                assert_eq!(
                    msg.get_error(),
                    CircuitError_Error::ERROR_RECIPIENT_NOT_IN_DIRECTORY
                );
                assert_eq!(msg.get_error_message(), "TEST");
                assert_eq!(msg.get_correlation_id(), "1234");
            },
        )
    }

    // Test that if the service the error message is meant for is not connected, the message is
    // dropped because there is no way to know where to send it. This test sends NetworkEcho
    #[test]
    fn test_circuit_error_handler_no_service() {
        // Set up dispatcher and mock sender
        let mock_sender = MockSender::new();
        let mut dispatcher = Dispatcher::new(Box::new(mock_sender.clone()));

        let table = RoutingTable::default();
        let reader: Box<dyn RoutingTableReader> = Box::new(table.clone());

        // Add circuit error handler to the the dispatcher
        let handler = CircuitErrorHandler::new("123".to_string(), reader);
        dispatcher.set_handler(Box::new(handler));

        // Create the circuit error message
        let mut circuit_error = CircuitError::new();
        circuit_error.set_service_id("abc".into());
        circuit_error.set_circuit_name("alpha".into());
        circuit_error.set_correlation_id("1234".into());
        circuit_error.set_error(CircuitError_Error::ERROR_RECIPIENT_NOT_IN_DIRECTORY);
        circuit_error.set_error_message("TEST".into());
        let error_bytes = circuit_error.write_to_bytes().unwrap();

        // dispatch the error message
        dispatcher
            .dispatch(
                PeerTokenPair::new(
                    PeerAuthorizationToken::from_peer_id("def"),
                    PeerAuthorizationToken::from_peer_id("345"),
                )
                .into(),
                &CircuitMessageType::CIRCUIT_ERROR_MESSAGE,
                error_bytes.clone(),
            )
            .unwrap();

        let mut network_echo = NetworkEcho::new();
        network_echo.set_payload(b"send_echo".to_vec());
        let mut network_msg = NetworkMessage::new();
        network_msg.set_payload(network_echo.write_to_bytes().unwrap());
        network_msg.set_message_type(NetworkMessageType::NETWORK_ECHO);
        mock_sender
            .send(
                PeerTokenPair::new(
                    PeerAuthorizationToken::from_peer_id("def"),
                    PeerAuthorizationToken::from_peer_id("345"),
                )
                .into(),
                network_msg.write_to_bytes().unwrap(),
            )
            .expect("Unable to send network echo");

        let (_, message) = mock_sender.next_outbound().expect("No message was sent");

        // verify that the message returned was an NetworkEcho, not a CircuitError
        let network_msg: NetworkMessage = Message::parse_from_bytes(&message).unwrap();

        assert_eq!(
            network_msg.get_message_type(),
            NetworkMessageType::NETWORK_ECHO
        );
    }

    fn assert_network_message<M: protobuf::Message, F: Fn(M)>(
        message: Vec<u8>,
        recipient: PeerTokenPair,
        expected_recipient: PeerTokenPair,
        expected_circuit_msg_type: CircuitMessageType,
        detail_assertions: F,
    ) {
        assert_eq!(expected_recipient, recipient);

        let network_msg: NetworkMessage = Message::parse_from_bytes(&message).unwrap();
        let circuit_msg: CircuitMessage =
            Message::parse_from_bytes(network_msg.get_payload()).unwrap();
        assert_eq!(expected_circuit_msg_type, circuit_msg.get_message_type(),);
        let circuit_msg: M = Message::parse_from_bytes(circuit_msg.get_payload()).unwrap();

        detail_assertions(circuit_msg);
    }

    #[derive(Clone)]
    struct MockSender {
        outbound: Arc<Mutex<VecDeque<(PeerId, Vec<u8>)>>>,
    }

    impl MockSender {
        fn new() -> Self {
            Self {
                outbound: Arc::new(Mutex::new(VecDeque::new())),
            }
        }

        fn next_outbound(&self) -> Option<(PeerId, Vec<u8>)> {
            self.outbound.lock().expect("lock was poisoned").pop_front()
        }
    }

    impl MessageSender<PeerId> for MockSender {
        fn send(&self, id: PeerId, message: Vec<u8>) -> Result<(), (PeerId, Vec<u8>)> {
            self.outbound
                .lock()
                .expect("lock was poisoned")
                .push_back((id, message));

            Ok(())
        }
    }
}
