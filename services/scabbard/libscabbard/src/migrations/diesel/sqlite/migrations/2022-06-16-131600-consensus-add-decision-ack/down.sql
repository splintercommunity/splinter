-- Copyright 2018-2022 Cargill Incorporated
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.
-- -----------------------------------------------------------------------------

PRAGMA foreign_keys=off;

CREATE TABLE IF NOT EXISTS new_consensus_2pc_context_participant (
    circuit_id                TEXT NOT NULL,
    service_id                TEXT NOT NULL,
    epoch                     BIGINT NOT NULL,
    process                   TEXT NOT NULL,
    vote                      TEXT
    CHECK ( vote IN ('TRUE' , 'FALSE') OR vote IS NULL ),
    PRIMARY KEY (circuit_id, service_id, process),
    FOREIGN KEY (circuit_id, service_id) REFERENCES scabbard_service(circuit_id, service_id)
);

CREATE TABLE IF NOT EXISTS new_consensus_2pc_update_context_action_participant (
    action_id                 INTEGER PRIMARY KEY,
    process                   TEXT NOT NULL,
    vote                      TEXT
    CHECK ( vote IN ('TRUE' , 'FALSE') OR vote IS NULL ),
    FOREIGN KEY (action_id) REFERENCES consensus_2pc_action(id) ON DELETE CASCADE,
    FOREIGN KEY (action_id) REFERENCES consensus_2pc_update_context_action(action_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS new_consensus_2pc_send_message_action (
    action_id                 INTEGER PRIMARY KEY,
    epoch                     BIGINT NOT NULL,
    receiver_service_id       TEXT NOT NULL,
    message_type              TEXT NOT NULL
    CHECK ( message_type IN ('VOTERESPONSE', 'DECISIONREQUEST', 'VOTEREQUEST', 'COMMIT', 'ABORT') ),
    vote_response             TEXT
    CHECK ( (vote_response IN ('TRUE', 'FALSE')) OR (message_type != 'VOTERESPONSE') ),
    vote_request              BINARY
    CHECK ( (vote_request IS NOT NULL) OR (message_type != 'VOTEREQUEST') ),
    FOREIGN KEY (action_id) REFERENCES consensus_2pc_action(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS new_consensus_2pc_deliver_event (
    event_id                  INTEGER PRIMARY KEY,
    epoch                     BIGINT NOT NULL,
    receiver_service_id       TEXT NOT NULL,
    message_type              TEXT NOT NULL
    CHECK ( message_type IN ('VOTERESPONSE', 'DECISIONREQUEST', 'VOTEREQUEST', 'COMMIT', 'ABORT') ),
    vote_response             TEXT
    CHECK ( (vote_response IN ('TRUE', 'FALSE')) OR (message_type != 'VOTERESPONSE') ),
    vote_request              BINARY
    CHECK ( (vote_request IS NOT NULL) OR (message_type != 'VOTEREQUEST') ),
    FOREIGN KEY (event_id) REFERENCES consensus_2pc_event(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS new_consensus_2pc_context (
    circuit_id                TEXT NOT NULL,
    service_id                TEXT NOT NULL,
    coordinator               TEXT NOT NULL,
    epoch                     BIGINT NOT NULL,
    last_commit_epoch         BIGINT,
    state                     TEXT NOT NULL
    CHECK ( state IN ( 'WAITINGFORSTART', 'VOTING', 'WAITINGFORVOTE', 'ABORT', 'COMMIT', 'WAITINGFORVOTEREQUEST', 'VOTED') ),
    vote_timeout_start        BIGINT
    CHECK ( (vote_timeout_start IS NOT NULL) OR ( state != 'VOTING') ),
    vote                      TEXT
    CHECK ( (vote IN ('TRUE' , 'FALSE')) OR ( state != 'VOTED') ),
    decision_timeout_start    BIGINT
    CHECK ( (decision_timeout_start IS NOT NULL) OR ( state != 'VOTED') ),
    PRIMARY KEY (circuit_id, service_id),
    FOREIGN KEY (circuit_id, service_id) REFERENCES scabbard_service(circuit_id, service_id)
);

CREATE TABLE IF NOT EXISTS new_consensus_2pc_update_context_action (
    action_id                 INTEGER PRIMARY KEY,
    coordinator               TEXT NOT NULL,
    epoch                     BIGINT NOT NULL,
    last_commit_epoch         BIGINT,
    state                     TEXT NOT NULL
    CHECK ( state IN ('WAITINGFORSTART', 'VOTING', 'WAITINGFORVOTE', 'ABORT', 'COMMIT', 'WAITINGFORVOTEREQUEST', 'VOTED') ),
    vote_timeout_start        BIGINT
    CHECK ( (vote_timeout_start IS NOT NULL) OR ( state != 'VOTING') ),
    vote                      TEXT
    CHECK ( (vote IN ('TRUE' , 'FALSE')) OR ( state != 'VOTED') ),
    decision_timeout_start    BIGINT
    CHECK ( (decision_timeout_start IS NOT NULL) OR ( state != 'VOTED') ),
    action_alarm  BIGINT,
    FOREIGN KEY (action_id) REFERENCES consensus_2pc_action(id) ON DELETE CASCADE
);

INSERT INTO new_consensus_2pc_context_participant
    (
        circuit_id,
        service_id,
        epoch,
        process,
        vote
    )
    SELECT
        circuit_id,
        service_id,
        epoch,
        process,
        vote
    FROM consensus_2pc_context_participant;

INSERT INTO new_consensus_2pc_update_context_action_participant
    (
        action_id,
        process,
        vote
    )
    SELECT
        action_id,
        process,
        vote
    FROM consensus_2pc_update_context_action_participant;

INSERT INTO new_consensus_2pc_send_message_action
    (
        action_id,
        epoch,
        alarm_type,
        receiver_service_id,
        message_type,
        vote_response,
        vote_request
    )
    SELECT
        action_id,
        epoch,
        alarm_type,
        receiver_service_id,
        message_type,
        vote_response,
        vote_request
    FROM consensus_2pc_send_message_action;

INSERT INTO new_consensus_2pc_deliver_event
    (
        event_id,
        epoch,
        receiver_service_id,
        message_type,
        vote_response,
        vote_request
    )
    SELECT
        event_id,
        epoch,
        receiver_service_id,
        message_type,
        vote_response,
        vote_request
    FROM consensus_2pc_deliver_event;

INSERT INTO new_consensus_2pc_context
    (
        circuit_id,
        service_id,
        coordinator,
        epoch,
        last_commit_epoch,
        state,
        vote_timeout_start,
        vote,
        decision_timeout_start
    )
    SELECT
        circuit_id,
        service_id,
        coordinator,
        epoch,
        last_commit_epoch,
        state,
        vote_timeout_start,
        vote,
        decision_timeout_start
    FROM consensus_2pc_context;

INSERT INTO new_consensus_2pc_update_context_action
    (
        action_id,
        coordinator,
        epoch,
        last_commit_epoch,
        state,
        vote_timeout_start,
        vote,
        decision_timeout_start,
        action_alarm
    )
    SELECT
        action_id,
        coordinator,
        epoch,
        last_commit_epoch,
        state,
        vote_timeout_start,
        vote,
        decision_timeout_start,
        action_alarm
    FROM consensus_2pc_update_context_action;

DROP TABLE consensus_2pc_context_participant;
DROP TABLE consensus_2pc_update_context_action_participant;
DROP TABLE consensus_2pc_send_message_action;
DROP TABLE consensus_2pc_deliver_event;
DROP TABLE consensus_2pc_context;
DROP TABLE consensus_2pc_update_context_action;

ALTER TABLE new_consensus_2pc_context_participant 
  RENAME TO consensus_2pc_context_participant;
ALTER TABLE new_consensus_2pc_update_context_action_participant 
  RENAME TO consensus_2pc_update_context_action_participant;
ALTER TABLE new_consensus_2pc_send_message_action
  RENAME TO consensus_2pc_send_message_action;
ALTER TABLE new_consensus_2pc_deliver_event
  RENAME TO consensus_2pc_deliver_event;
ALTER TABLE new_consensus_2pc_context
  RENAME TO consensus_2pc_context;
ALTER TABLE new_consensus_2pc_update_context_action
  RENAME TO consensus_2pc_update_context_action;

PRAGMA foreign_keys=on;
