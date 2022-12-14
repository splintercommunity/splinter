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

use crossbeam_channel::Sender;

use crate::events::ReactorError;

use super::ReactorMessage;

pub struct ReactorShutdownSignaler {
    pub(super) sender: Sender<ReactorMessage>,
}

impl ReactorShutdownSignaler {
    pub fn signal_shutdown(&self) -> Result<(), ReactorError> {
        self.sender.send(ReactorMessage::Stop).map_err(|_| {
            ReactorError::ReactorShutdownError("Failed to send shutdown message".to_string())
        })
    }
}
