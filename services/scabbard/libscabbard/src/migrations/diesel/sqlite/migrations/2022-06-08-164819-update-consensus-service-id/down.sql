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

-- Recreate the tables without the circuit_id columns
CREATE TABLE IF NOT EXISTS new_scabbard_service (
    service_id       TEXT PRIMARY KEY NOT NULL,
    status           TEXT NOT NULL
    CHECK ( status IN ('PREPARED', 'FINALIZED', 'RETIRED') ),
    consensus                 scabbard_consensus NOT NULL
);

CREATE TABLE IF NOT EXISTS new_scabbard_peer (
    service_id                TEXT NOT NULL,
    peer_service_id           TEXT,
    PRIMARY KEY(service_id, peer_service_id),
    FOREIGN KEY(service_id) REFERENCES scabbard_service(service_id)
);

CREATE TABLE IF NOT EXISTS new_scabbard_v3_commit_history (
    service_id                TEXT NOT NULL,
    epoch                     INTEGER NOT NULL,
    value                     TEXT NOT NULL,
    decision                  TEXT,
    CHECK ( decision IN ('COMMIT', 'ABORT') ),
    PRIMARY KEY (service_id, epoch)
);

CREATE TABLE IF NOT EXISTS new_consensus_2pc_context (
    service_id                TEXT PRIMARY KEY,
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
    CHECK ( (decision_timeout_start IS NOT NULL) OR ( state != 'VOTED') )
);

CREATE TABLE IF NOT EXISTS new_consensus_2pc_context_participant (
    service_id                TEXT NOT NULL,
    epoch                     BIGINT NOT NULL,
    process                   TEXT NOT NULL,
    vote                      TEXT
    CHECK ( vote IN ('TRUE' , 'FALSE') OR vote IS NULL ),
    PRIMARY KEY (service_id, process),
    FOREIGN KEY (service_id) REFERENCES consensus_2pc_context(service_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS new_consensus_2pc_action (
    id                        INTEGER PRIMARY KEY AUTOINCREMENT,
    service_id                TEXT NOT NULL,
    created_at                TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    executed_at               BIGINT,
    FOREIGN KEY (service_id) REFERENCES consensus_2pc_context(service_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS new_consensus_2pc_event (
    id                        INTEGER PRIMARY KEY AUTOINCREMENT,
    service_id                TEXT NOT NULL,
    created_at                TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    executed_at               BIGINT,
    event_type                TEXT NOT NULL
    CHECK ( event_type IN ('ALARM', 'DELIVER', 'START', 'VOTE') )
);

CREATE TABLE IF NOT EXISTS new_scabbard_alarm (
    service_id                TEXT NOT NULL,
    alarm_type                TEXT NOT NULL
    CHECK ( alarm_type IN ('TWOPHASECOMMIT')),
    alarm                     BIGINT NOT NULL,
    FOREIGN KEY (service_id) REFERENCES scabbard_service(service_id) ON DELETE CASCADE,
    PRIMARY KEY (service_id, alarm_type)
);

-- Move data from the old tables into the updated tables
INSERT INTO new_scabbard_service
    (
        service_id,
        status,
        consensus
    )
    SELECT
        circuit_id || '::' || service_id,
        status,
        consensus
    FROM scabbard_service;

INSERT INTO new_scabbard_peer
    (
        service_id,
        peer_service_id
    )
    SELECT
        circuit_id || '::' || service_id,
        peer_service_id
    FROM scabbard_peer;

INSERT INTO new_scabbard_v3_commit_history
    (
        service_id,
        epoch,
        value,
        decision
    )
    SELECT
        circuit_id || '::' || service_id,
        epoch,
        value,
        decision
    FROM scabbard_v3_commit_history;

INSERT INTO new_consensus_2pc_context
    (
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
        circuit_id || '::' || service_id,
        coordinator,
        epoch,
        last_commit_epoch,
        state,
        vote_timeout_start,
        vote,
        decision_timeout_start
    FROM consensus_2pc_context;

INSERT INTO new_consensus_2pc_context_participant
    (
        service_id,
        epoch,
        process,
        vote
    )
    SELECT
        circuit_id || '::' || service_id,
        epoch,
        process,
        vote
    FROM consensus_2pc_context_participant;

INSERT INTO new_consensus_2pc_action
    (
        id,
        service_id,
        created_at,
        executed_at
    )
    SELECT
        id,
        circuit_id || '::' || service_id,
        created_at,
        executed_at
    FROM consensus_2pc_action;

INSERT INTO new_consensus_2pc_event
    (
        id,
        service_id,
        executed_at,
        created_at,
        event_type
    )
    SELECT
        id,
        circuit_id || '::' || service_id,
        executed_at,
        created_at,
        event_type
    FROM consensus_2pc_event;

INSERT INTO new_scabbard_alarm
    (
        service_id,
        alarm_type,
        alarm
    )
    SELECT
        circuit_id || '::' || service_id,
        alarm_type,
        alarm
    FROM scabbard_alarm;

-- Drop the old tables
DROP TABLE scabbard_service;
DROP TABLE scabbard_peer;
DROP TABLE scabbard_v3_commit_history;
DROP TABLE consensus_2pc_context;
DROP TABLE consensus_2pc_context_participant;
DROP TABLE consensus_2pc_action;
DROP TABLE consensus_2pc_event;
DROP TABLE scabbard_alarm;

-- Rename the new tables
ALTER TABLE new_scabbard_service RENAME TO scabbard_service;
ALTER TABLE new_scabbard_peer RENAME TO scabbard_peer;
ALTER TABLE new_scabbard_v3_commit_history RENAME TO scabbard_v3_commit_history;
ALTER TABLE new_consensus_2pc_context RENAME TO consensus_2pc_context;
ALTER TABLE new_consensus_2pc_context_participant RENAME TO consensus_2pc_context_participant;
ALTER TABLE new_consensus_2pc_action RENAME TO consensus_2pc_action;
ALTER TABLE new_consensus_2pc_event RENAME TO consensus_2pc_event;
ALTER TABLE new_scabbard_alarm RENAME TO scabbard_alarm;

PRAGMA foreign_keys=on;
