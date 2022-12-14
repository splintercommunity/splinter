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

use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use openssl::ssl::SslAcceptor;
use tungstenite::{accept, handshake::HandshakeError};

use crate::transport::{AcceptError, Connection, Listener};

use super::connection::WsConnection;
use super::transport::WSS_PROTOCOL_PREFIX;
use super::transport::WS_PROTOCOL_PREFIX;

pub(super) struct WsListener {
    listener: TcpListener,
    local_endpoint: String,
    acceptor: Option<SslAcceptor>,
}

impl WsListener {
    pub fn new(
        listener: TcpListener,
        local_endpoint: String,
        acceptor: Option<SslAcceptor>,
    ) -> Self {
        WsListener {
            listener,
            local_endpoint,
            acceptor,
        }
    }
}

impl Listener for WsListener {
    fn accept(&mut self) -> Result<Box<dyn Connection>, AcceptError> {
        let (stream, _) = self.listener.accept()?;

        if let Some(acceptor) = &self.acceptor {
            let remote_endpoint = format!("{}{}", WSS_PROTOCOL_PREFIX, stream.peer_addr()?);

            let websocket = accept(acceptor.accept(stream)?).map_or_else(
                {
                    |mut handshake_err| loop {
                        match handshake_err {
                            HandshakeError::Interrupted(mid_handshake) => {
                                thread::sleep(Duration::from_millis(100));
                                match mid_handshake.handshake() {
                                    Ok(ok) => break Ok(ok),
                                    Err(err) => handshake_err = err,
                                }
                            }
                            HandshakeError::Failure(err) => break Err(err),
                        }
                    }
                },
                Ok,
            )?;

            Ok(Box::new(WsConnection::new(
                websocket,
                remote_endpoint,
                self.local_endpoint.clone(),
            )))
        } else {
            let remote_endpoint = format!("{}{}", WS_PROTOCOL_PREFIX, stream.peer_addr()?);

            let websocket = accept(stream).map_or_else(
                {
                    |mut handshake_err| loop {
                        match handshake_err {
                            HandshakeError::Interrupted(mid_handshake) => {
                                thread::sleep(Duration::from_millis(100));
                                match mid_handshake.handshake() {
                                    Ok(ok) => break Ok(ok),
                                    Err(err) => handshake_err = err,
                                }
                            }
                            HandshakeError::Failure(err) => break Err(err),
                        }
                    }
                },
                Ok,
            )?;

            Ok(Box::new(WsConnection::new(
                websocket,
                remote_endpoint,
                self.local_endpoint.clone(),
            )))
        }
    }

    fn endpoint(&self) -> String {
        self.local_endpoint.clone()
    }
}

impl From<tungstenite::error::Error> for AcceptError {
    fn from(err: tungstenite::error::Error) -> Self {
        match err {
            tungstenite::error::Error::Io(io) => AcceptError::from(io),
            _ => AcceptError::ProtocolError(format!("handshake failure: {}", err)),
        }
    }
}
