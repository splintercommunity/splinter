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

pub mod builders;

use protobuf::{self, RepeatedField};
use std::convert::TryInto;

use crate::admin::error::MarshallingError;
use crate::admin::store;
use crate::admin::store::EventType;
use crate::hex::{as_hex, deserialize_hex};
use crate::protos::admin::{self, CircuitCreateRequest};

pub const UNSET_CIRCUIT_VERSION: i32 = 1;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct CreateCircuit {
    pub circuit_id: String,
    pub roster: Vec<SplinterService>,
    pub members: Vec<SplinterNode>,
    pub authorization_type: AuthorizationType,
    #[serde(default)]
    pub persistence: PersistenceType,
    pub durability: DurabilityType,
    #[serde(default)]
    pub routes: RouteType,
    pub circuit_management_type: String,
    #[serde(serialize_with = "as_hex")]
    #[serde(deserialize_with = "deserialize_hex")]
    #[serde(default)]
    pub application_metadata: Vec<u8>,
    #[serde(default)]
    pub comments: Option<String>,
    pub display_name: Option<String>,
    #[serde(default)]
    pub circuit_version: i32,
    pub circuit_status: CircuitStatus,
}

impl CreateCircuit {
    pub fn from_proto(mut proto: admin::Circuit) -> Result<Self, MarshallingError> {
        let authorization_type = match proto.get_authorization_type() {
            admin::Circuit_AuthorizationType::TRUST_AUTHORIZATION => AuthorizationType::Trust,
            admin::Circuit_AuthorizationType::CHALLENGE_AUTHORIZATION => {
                AuthorizationType::Challenge
            }
            _ => {
                return Err(MarshallingError::UnsetField(
                    "Unsupported authorization type".to_string(),
                ));
            }
        };

        let persistence = match proto.get_persistence() {
            admin::Circuit_PersistenceType::ANY_PERSISTENCE => PersistenceType::Any,
            admin::Circuit_PersistenceType::UNSET_PERSISTENCE_TYPE => {
                return Err(MarshallingError::UnsetField(
                    "Unset persistence type".to_string(),
                ));
            }
        };

        let durability = match proto.get_durability() {
            admin::Circuit_DurabilityType::NO_DURABILITY => DurabilityType::NoDurability,
            admin::Circuit_DurabilityType::UNSET_DURABILITY_TYPE => {
                return Err(MarshallingError::UnsetField(
                    "Unset durability type".to_string(),
                ));
            }
        };

        let routes = match proto.get_routes() {
            admin::Circuit_RouteType::ANY_ROUTE => RouteType::Any,
            admin::Circuit_RouteType::UNSET_ROUTE_TYPE => {
                return Err(MarshallingError::UnsetField("Unset route type".to_string()));
            }
        };

        let circuit_status = match proto.get_circuit_status() {
            admin::Circuit_CircuitStatus::ACTIVE => CircuitStatus::Active,
            admin::Circuit_CircuitStatus::DISBANDED => CircuitStatus::Disbanded,
            admin::Circuit_CircuitStatus::ABANDONED => CircuitStatus::Abandoned,
            admin::Circuit_CircuitStatus::UNSET_CIRCUIT_STATUS => {
                debug!("Defaulting `UNSET_CIRCUIT_STATUS` of proposed circuit to `Active`");
                CircuitStatus::Active
            }
        };

        let display_name = if proto.get_display_name().is_empty() {
            None
        } else {
            Some(proto.get_display_name().into())
        };

        let comments = if proto.get_comments().is_empty() {
            None
        } else {
            Some(proto.get_comments().into())
        };

        let circuit_version = if proto.get_circuit_version() == 0 {
            UNSET_CIRCUIT_VERSION
        } else {
            proto.get_circuit_version()
        };

        Ok(Self {
            circuit_id: proto.take_circuit_id(),
            roster: proto
                .take_roster()
                .into_iter()
                .map(SplinterService::from_proto)
                .collect::<Result<Vec<SplinterService>, MarshallingError>>()?,
            members: proto
                .take_members()
                .into_iter()
                .map(SplinterNode::from_proto)
                .collect::<Result<Vec<SplinterNode>, MarshallingError>>()?,
            authorization_type,
            persistence,
            durability,
            routes,
            circuit_management_type: proto.take_circuit_management_type(),
            application_metadata: proto.take_application_metadata(),
            comments,
            display_name,
            circuit_version,
            circuit_status,
        })
    }

    pub fn into_proto(self) -> Result<CircuitCreateRequest, MarshallingError> {
        let mut circuit = admin::Circuit::new();

        circuit.set_circuit_id(self.circuit_id);
        circuit.set_roster(RepeatedField::from_vec(
            self.roster
                .into_iter()
                .map(SplinterService::into_proto)
                .collect(),
        ));
        circuit.set_members(RepeatedField::from_vec(
            self.members
                .into_iter()
                .map(SplinterNode::into_proto)
                .collect(),
        ));

        circuit.set_circuit_management_type(self.circuit_management_type);
        circuit.set_application_metadata(self.application_metadata);

        if let Some(comments) = self.comments {
            circuit.set_comments(comments);
        }

        if let Some(display_name) = self.display_name {
            circuit.set_display_name(display_name);
        }

        if self.circuit_version != UNSET_CIRCUIT_VERSION {
            circuit.set_circuit_version(self.circuit_version);
        }

        match self.authorization_type {
            AuthorizationType::Trust => {
                circuit
                    .set_authorization_type(admin::Circuit_AuthorizationType::TRUST_AUTHORIZATION);
            }
            AuthorizationType::Challenge => {
                circuit.set_authorization_type(
                    admin::Circuit_AuthorizationType::CHALLENGE_AUTHORIZATION,
                );
            }
        };

        match self.persistence {
            PersistenceType::Any => {
                circuit.set_persistence(admin::Circuit_PersistenceType::ANY_PERSISTENCE);
            }
        };
        match self.durability {
            DurabilityType::NoDurability => {
                circuit.set_durability(admin::Circuit_DurabilityType::NO_DURABILITY);
            }
        };

        match self.routes {
            RouteType::Any => circuit.set_routes(admin::Circuit_RouteType::ANY_ROUTE),
        };

        if self.circuit_version > UNSET_CIRCUIT_VERSION {
            match self.circuit_status {
                CircuitStatus::Active => {
                    circuit.set_circuit_status(admin::Circuit_CircuitStatus::ACTIVE);
                }
                CircuitStatus::Disbanded => {
                    circuit.set_circuit_status(admin::Circuit_CircuitStatus::DISBANDED);
                }
                CircuitStatus::Abandoned => {
                    circuit.set_circuit_status(admin::Circuit_CircuitStatus::ABANDONED);
                }
            };
        }

        let mut create_request = CircuitCreateRequest::new();
        create_request.set_circuit(circuit);

        Ok(create_request)
    }
}

impl TryInto<admin::Circuit> for CreateCircuit {
    type Error = MarshallingError;

    fn try_into(self) -> Result<admin::Circuit, Self::Error> {
        let mut circuit = admin::Circuit::new();

        circuit.set_circuit_id(self.circuit_id);
        circuit.set_roster(RepeatedField::from_vec(
            self.roster
                .into_iter()
                .map(SplinterService::into_proto)
                .collect(),
        ));
        circuit.set_members(RepeatedField::from_vec(
            self.members
                .into_iter()
                .map(SplinterNode::into_proto)
                .collect(),
        ));

        circuit.set_circuit_management_type(self.circuit_management_type);
        circuit.set_application_metadata(self.application_metadata);

        if let Some(comments) = self.comments {
            circuit.set_comments(comments);
        }

        if let Some(display_name) = self.display_name {
            circuit.set_display_name(display_name);
        }

        if self.circuit_version != UNSET_CIRCUIT_VERSION {
            circuit.set_circuit_version(self.circuit_version);
        }

        match self.authorization_type {
            AuthorizationType::Trust => {
                circuit
                    .set_authorization_type(admin::Circuit_AuthorizationType::TRUST_AUTHORIZATION);
            }
            AuthorizationType::Challenge => {
                circuit.set_authorization_type(
                    admin::Circuit_AuthorizationType::CHALLENGE_AUTHORIZATION,
                );
            }
        };

        match self.persistence {
            PersistenceType::Any => {
                circuit.set_persistence(admin::Circuit_PersistenceType::ANY_PERSISTENCE);
            }
        };
        match self.durability {
            DurabilityType::NoDurability => {
                circuit.set_durability(admin::Circuit_DurabilityType::NO_DURABILITY);
            }
        };

        match self.routes {
            RouteType::Any => circuit.set_routes(admin::Circuit_RouteType::ANY_ROUTE),
        };

        match self.circuit_status {
            CircuitStatus::Active => {
                circuit.set_circuit_status(admin::Circuit_CircuitStatus::ACTIVE);
            }
            CircuitStatus::Disbanded => {
                circuit.set_circuit_status(admin::Circuit_CircuitStatus::DISBANDED);
            }
            CircuitStatus::Abandoned => {
                circuit.set_circuit_status(admin::Circuit_CircuitStatus::ABANDONED);
            }
        };

        Ok(circuit)
    }
}

/// Determines if a circuit ID is valid. A valid circuit ID is an 11 character string composed of
/// two, 5 character base62 strings joined with a '-' (example: abcDE-F0123).
pub fn is_valid_circuit_id(circuit_id: &str) -> bool {
    let mut split = circuit_id.splitn(2, '-');
    let is_two_parts = split.clone().count() == 2;
    let are_parts_valid = split.all(|part| {
        let is_correct_len = part.len() == 5;
        let is_base62 = part.chars().all(|c| c.is_ascii_alphanumeric());
        is_correct_len && is_base62
    });
    is_two_parts && are_parts_valid
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum AuthorizationType {
    Trust,
    Challenge,
}

impl From<&store::AuthorizationType> for AuthorizationType {
    fn from(store_enum: &store::AuthorizationType) -> Self {
        match *store_enum {
            store::AuthorizationType::Trust => AuthorizationType::Trust,
            store::AuthorizationType::Challenge => AuthorizationType::Challenge,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum PersistenceType {
    Any,
}

impl Default for PersistenceType {
    fn default() -> Self {
        PersistenceType::Any
    }
}

impl From<&store::PersistenceType> for PersistenceType {
    fn from(store_enum: &store::PersistenceType) -> Self {
        match *store_enum {
            store::PersistenceType::Any => PersistenceType::Any,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum DurabilityType {
    NoDurability,
}

impl From<&store::DurabilityType> for DurabilityType {
    fn from(store_enum: &store::DurabilityType) -> Self {
        match *store_enum {
            store::DurabilityType::NoDurability => DurabilityType::NoDurability,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum RouteType {
    Any,
}

impl Default for RouteType {
    fn default() -> Self {
        RouteType::Any
    }
}

impl From<&store::RouteType> for RouteType {
    fn from(store_enum: &store::RouteType) -> Self {
        match *store_enum {
            store::RouteType::Any => RouteType::Any,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum CircuitStatus {
    Active,
    Disbanded,
    Abandoned,
}

impl Default for CircuitStatus {
    fn default() -> Self {
        CircuitStatus::Active
    }
}

impl From<&store::CircuitStatus> for CircuitStatus {
    fn from(store_enum: &store::CircuitStatus) -> Self {
        match *store_enum {
            store::CircuitStatus::Active => CircuitStatus::Active,
            store::CircuitStatus::Disbanded => CircuitStatus::Disbanded,
            store::CircuitStatus::Abandoned => CircuitStatus::Abandoned,
        }
    }
}

impl std::fmt::Display for CircuitStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CircuitStatus::Active => f.write_str("Active"),
            CircuitStatus::Disbanded => f.write_str("Disbanded"),
            CircuitStatus::Abandoned => f.write_str("Abandoned"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SplinterNode {
    pub node_id: String,
    pub endpoints: Vec<String>,
    pub public_key: Option<Vec<u8>>,
}

impl SplinterNode {
    pub fn into_proto(self) -> admin::SplinterNode {
        let mut proto = admin::SplinterNode::new();

        proto.set_node_id(self.node_id);
        proto.set_endpoints(self.endpoints.into());

        if let Some(public_key) = self.public_key {
            proto.set_public_key(public_key);
        }

        proto
    }

    pub fn from_proto(mut proto: admin::SplinterNode) -> Result<Self, MarshallingError> {
        let public_key = {
            let public_key = proto.take_public_key();
            if public_key.is_empty() {
                None
            } else {
                Some(public_key)
            }
        };

        Ok(Self {
            node_id: proto.take_node_id(),
            endpoints: proto.take_endpoints().into(),
            public_key,
        })
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SplinterService {
    pub service_id: String,
    pub service_type: String,
    pub allowed_nodes: Vec<String>,
    #[serde(default)]
    pub arguments: Vec<(String, String)>,
}

impl SplinterService {
    pub fn into_proto(self) -> admin::SplinterService {
        let mut proto = admin::SplinterService::new();
        proto.set_service_id(self.service_id);
        proto.set_service_type(self.service_type);
        proto.set_allowed_nodes(RepeatedField::from_vec(self.allowed_nodes));
        proto.set_arguments(RepeatedField::from_vec(
            self.arguments
                .into_iter()
                .map(|(k, v)| {
                    let mut argument = admin::SplinterService_Argument::new();
                    argument.set_key(k);
                    argument.set_value(v);
                    argument
                })
                .collect(),
        ));

        proto
    }

    pub fn from_proto(mut proto: admin::SplinterService) -> Result<Self, MarshallingError> {
        Ok(Self {
            service_id: proto.take_service_id(),
            service_type: proto.take_service_type(),
            allowed_nodes: proto
                .take_allowed_nodes()
                .into_iter()
                .map(String::from)
                .collect(),
            arguments: proto
                .take_arguments()
                .into_iter()
                .map(|mut argument| (argument.take_key(), argument.take_value()))
                .collect(),
        })
    }
}

/// Determines if a service ID is valid. A valid service ID is a 4 character base62 string.
pub fn is_valid_service_id(service_id: &str) -> bool {
    let is_correct_len = service_id.len() == 4;
    let is_base62 = service_id.chars().all(|c| c.is_ascii_alphanumeric());
    is_correct_len && is_base62
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct CircuitProposal {
    pub proposal_type: ProposalType,
    pub circuit_id: String,
    pub circuit_hash: String,
    pub circuit: CreateCircuit,
    pub votes: Vec<VoteRecord>,
    #[serde(serialize_with = "as_hex")]
    #[serde(deserialize_with = "deserialize_hex")]
    pub requester: Vec<u8>,
    pub requester_node_id: String,
}

impl CircuitProposal {
    pub fn from_proto(mut proto: admin::CircuitProposal) -> Result<Self, MarshallingError> {
        let proposal_type = match proto.get_proposal_type() {
            admin::CircuitProposal_ProposalType::CREATE => ProposalType::Create,
            admin::CircuitProposal_ProposalType::UPDATE_ROSTER => ProposalType::UpdateRoster,
            admin::CircuitProposal_ProposalType::ADD_NODE => ProposalType::AddNode,
            admin::CircuitProposal_ProposalType::REMOVE_NODE => ProposalType::RemoveNode,
            admin::CircuitProposal_ProposalType::DISBAND => ProposalType::Disband,
            admin::CircuitProposal_ProposalType::UNSET_PROPOSAL_TYPE => {
                return Err(MarshallingError::UnsetField(
                    "Unset proposal type".to_string(),
                ));
            }
        };

        let votes = proto
            .take_votes()
            .into_iter()
            .map(VoteRecord::from_proto)
            .collect::<Result<Vec<VoteRecord>, MarshallingError>>()?;

        Ok(Self {
            proposal_type,
            circuit_id: proto.take_circuit_id(),
            circuit_hash: proto.take_circuit_hash(),
            circuit: CreateCircuit::from_proto(proto.take_circuit_proposal())?,
            votes,
            requester: proto.take_requester(),
            requester_node_id: proto.take_requester_node_id(),
        })
    }

    pub fn into_proto(self) -> Result<admin::CircuitProposal, MarshallingError> {
        let proposal_type = match self.proposal_type {
            ProposalType::Create => admin::CircuitProposal_ProposalType::CREATE,
            ProposalType::UpdateRoster => admin::CircuitProposal_ProposalType::UPDATE_ROSTER,
            ProposalType::AddNode => admin::CircuitProposal_ProposalType::ADD_NODE,
            ProposalType::RemoveNode => admin::CircuitProposal_ProposalType::REMOVE_NODE,
            ProposalType::Disband => admin::CircuitProposal_ProposalType::DISBAND,
        };

        let votes = self
            .votes
            .into_iter()
            .map(|vote| vote.into_proto())
            .collect::<Vec<admin::CircuitProposal_VoteRecord>>();

        let mut circuit_request = self.circuit.into_proto()?;

        let mut proposal = admin::CircuitProposal::new();
        proposal.set_proposal_type(proposal_type);
        proposal.set_circuit_id(self.circuit_id.to_string());
        proposal.set_circuit_hash(self.circuit_hash.to_string());
        proposal.set_circuit_proposal(circuit_request.take_circuit());
        proposal.set_votes(RepeatedField::from_vec(votes));
        proposal.set_requester(self.requester.to_vec());
        proposal.set_requester_node_id(self.requester_node_id);

        Ok(proposal)
    }
}

impl From<store::CircuitProposal> for CircuitProposal {
    fn from(store_proposal: store::CircuitProposal) -> Self {
        let proposal_type = match store_proposal.proposal_type() {
            store::ProposalType::Create => ProposalType::Create,
            store::ProposalType::UpdateRoster => ProposalType::UpdateRoster,
            store::ProposalType::AddNode => ProposalType::AddNode,
            store::ProposalType::RemoveNode => ProposalType::RemoveNode,
            store::ProposalType::Disband => ProposalType::Disband,
        };

        let store_circuit = store_proposal.circuit();

        let circuit = CreateCircuit {
            circuit_id: store_proposal.circuit_id().into(),
            roster: store_circuit
                .roster()
                .iter()
                .map(|service| SplinterService {
                    service_id: service.service_id().into(),
                    service_type: service.service_type().into(),
                    allowed_nodes: vec![service.node_id().to_string()],
                    arguments: service
                        .arguments()
                        .iter()
                        .map(|(key, value)| (key.to_string(), value.to_string()))
                        .collect(),
                })
                .collect::<Vec<SplinterService>>(),
            members: store_circuit
                .members()
                .iter()
                .map(|node| SplinterNode {
                    node_id: node.node_id().to_string(),
                    endpoints: node.endpoints().to_vec(),
                    public_key: node
                        .public_key()
                        .clone()
                        .map(|public_key| public_key.into_bytes()),
                })
                .collect::<Vec<SplinterNode>>(),
            authorization_type: AuthorizationType::from(store_circuit.authorization_type()),
            persistence: PersistenceType::Any,
            durability: DurabilityType::NoDurability,
            routes: RouteType::Any,
            circuit_management_type: store_circuit.circuit_management_type().into(),
            application_metadata: store_circuit
                .application_metadata()
                .clone()
                .unwrap_or_default(),
            comments: store_circuit.comments().clone(),
            display_name: store_circuit.display_name().clone(),
            circuit_version: store_circuit.circuit_version(),
            circuit_status: CircuitStatus::from(&store_circuit.circuit_status().clone()),
        };

        Self {
            proposal_type,
            circuit_id: store_proposal.circuit_id().into(),
            circuit_hash: store_proposal.circuit_hash().into(),
            circuit,
            votes: store_proposal
                .votes()
                .iter()
                .map(|vote_record| {
                    let vote = match vote_record.vote() {
                        store::Vote::Accept => Vote::Accept,
                        store::Vote::Reject => Vote::Reject,
                    };

                    VoteRecord {
                        public_key: vote_record.public_key().as_slice().to_vec(),
                        vote,
                        voter_node_id: vote_record.voter_node_id().into(),
                    }
                })
                .collect(),
            requester: store_proposal.requester().as_slice().to_vec(),
            requester_node_id: store_proposal.requester_node_id().into(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ProposalType {
    Create,
    UpdateRoster,
    AddNode,
    RemoveNode,
    Disband,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct CircuitProposalVote {
    pub circuit_id: String,
    pub circuit_hash: String,
    pub vote: Vote,
}

impl CircuitProposalVote {
    pub fn from_proto(mut proto: admin::CircuitProposalVote) -> Result<Self, MarshallingError> {
        let vote = match proto.get_vote() {
            admin::CircuitProposalVote_Vote::ACCEPT => Vote::Accept,
            admin::CircuitProposalVote_Vote::REJECT => Vote::Reject,
            admin::CircuitProposalVote_Vote::UNSET_VOTE => {
                return Err(MarshallingError::UnsetField("Unset vote".to_string()));
            }
        };

        Ok(CircuitProposalVote {
            circuit_id: proto.take_circuit_id(),
            circuit_hash: proto.take_circuit_hash(),
            vote,
        })
    }

    pub fn into_proto(self) -> admin::CircuitProposalVote {
        let mut vote = admin::CircuitProposalVote::new();
        vote.set_circuit_id(self.circuit_id);
        vote.set_circuit_hash(self.circuit_hash);
        match self.vote {
            Vote::Accept => vote.set_vote(admin::CircuitProposalVote_Vote::ACCEPT),
            Vote::Reject => vote.set_vote(admin::CircuitProposalVote_Vote::REJECT),
        }
        vote
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct VoteRecord {
    #[serde(serialize_with = "as_hex")]
    #[serde(deserialize_with = "deserialize_hex")]
    pub public_key: Vec<u8>,
    pub vote: Vote,
    pub voter_node_id: String,
}

impl VoteRecord {
    fn from_proto(mut proto: admin::CircuitProposal_VoteRecord) -> Result<Self, MarshallingError> {
        let vote = match proto.get_vote() {
            admin::CircuitProposalVote_Vote::ACCEPT => Vote::Accept,
            admin::CircuitProposalVote_Vote::REJECT => Vote::Reject,
            admin::CircuitProposalVote_Vote::UNSET_VOTE => {
                return Err(MarshallingError::UnsetField("Unset vote".to_string()));
            }
        };

        Ok(Self {
            public_key: proto.take_public_key(),
            vote,
            voter_node_id: proto.take_voter_node_id(),
        })
    }

    fn into_proto(self) -> admin::CircuitProposal_VoteRecord {
        let vote = match self.vote {
            Vote::Accept => admin::CircuitProposalVote_Vote::ACCEPT,
            Vote::Reject => admin::CircuitProposalVote_Vote::REJECT,
        };

        let mut vote_record = admin::CircuitProposal_VoteRecord::new();
        vote_record.set_vote(vote);
        vote_record.set_public_key(self.public_key);
        vote_record.set_voter_node_id(self.voter_node_id);

        vote_record
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Vote {
    Accept,
    Reject,
}

/// Represents the `requester`'s public key associated with an `AdminServiceEvent`
pub type PublicKey = Vec<u8>;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "eventType", content = "message")]
pub enum AdminServiceEvent {
    ProposalSubmitted(CircuitProposal),
    ProposalVote((CircuitProposal, PublicKey)),
    ProposalAccepted((CircuitProposal, PublicKey)),
    ProposalRejected((CircuitProposal, PublicKey)),
    CircuitReady(CircuitProposal),
    CircuitDisbanded(CircuitProposal),
}

impl AdminServiceEvent {
    pub fn proposal(&self) -> &CircuitProposal {
        match self {
            AdminServiceEvent::ProposalSubmitted(proposal) => proposal,
            AdminServiceEvent::ProposalVote((proposal, _)) => proposal,
            AdminServiceEvent::ProposalAccepted((proposal, _)) => proposal,
            AdminServiceEvent::ProposalRejected((proposal, _)) => proposal,
            AdminServiceEvent::CircuitReady(proposal) => proposal,
            AdminServiceEvent::CircuitDisbanded(proposal) => proposal,
        }
    }
}

impl From<&store::AdminServiceEvent> for AdminServiceEvent {
    fn from(event_entry: &store::AdminServiceEvent) -> Self {
        let admin_proposal = CircuitProposal::from(event_entry.proposal().clone());
        match event_entry.event_type() {
            EventType::ProposalSubmitted => AdminServiceEvent::ProposalSubmitted(admin_proposal),
            EventType::ProposalVote { requester } => {
                AdminServiceEvent::ProposalVote((admin_proposal, requester.to_vec()))
            }
            EventType::ProposalAccepted { requester } => {
                AdminServiceEvent::ProposalAccepted((admin_proposal, requester.to_vec()))
            }
            EventType::ProposalRejected { requester } => {
                AdminServiceEvent::ProposalRejected((admin_proposal, requester.to_vec()))
            }
            EventType::CircuitReady => AdminServiceEvent::CircuitReady(admin_proposal),
            EventType::CircuitDisbanded => AdminServiceEvent::CircuitDisbanded(admin_proposal),
        }
    }
}
