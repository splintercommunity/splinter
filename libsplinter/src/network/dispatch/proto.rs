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

use protobuf::Message;

use crate::network::dispatch::{DispatchError, FromMessageBytes};
use crate::protos::prelude::*;

// Implements FromMessageBytes for all protobuf Message values.
impl<M> FromMessageBytes for M
where
    M: Message + Sized,
{
    fn from_message_bytes(message_bytes: &[u8]) -> Result<Self, DispatchError> {
        Message::parse_from_bytes(message_bytes)
            .map_err(|err| DispatchError::DeserializationError(err.to_string()))
    }
}

impl From<ProtoConversionError> for DispatchError {
    fn from(err: ProtoConversionError) -> DispatchError {
        match err {
            ProtoConversionError::DeserializationError(s) => DispatchError::DeserializationError(s),
            ProtoConversionError::SerializationError(s) => DispatchError::SerializationError(s),
            // This is detected due to a protobuf that does not properly have fields set correctly.
            ProtoConversionError::InvalidTypeError(s) => DispatchError::DeserializationError(s),
        }
    }
}
