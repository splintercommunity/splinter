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

ALTER TABLE consensus_2pc_context_participant
  ADD COLUMN decision_ack BOOLEAN NOT NULL Default FALSE;

ALTER TABLE consensus_2pc_update_context_action_participant
  ADD COLUMN decision_ack BOOLEAN NOT NULL Default FALSE;

ALTER TYPE message_type ADD VALUE IF NOT EXISTS 'DECISION_ACK';
ALTER TYPE deliver_event_message_type ADD VALUE IF NOT EXISTS 'DECISION_ACK';

ALTER TYPE context_state ADD VALUE IF NOT EXISTS 'WAITING_FOR_DECISION_ACK';

COMMIT;

ALTER TABLE consensus_2pc_context ADD COLUMN ack_timeout_start BIGINT;

ALTER TABLE consensus_2pc_context ADD CONSTRAINT ack_timeout_start_check
  CHECK ( (ack_timeout_start IS NOT NULL) OR (state != 'WAITING_FOR_DECISION_ACK') );
