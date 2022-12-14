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

#[derive(Debug)]
pub struct ProposalStoreError {
    context: String,
    source: Option<Box<dyn std::error::Error + 'static>>,
}

impl std::error::Error for ProposalStoreError {}

impl ProposalStoreError {
    pub fn new(context: &str) -> Self {
        Self {
            context: context.into(),
            source: None,
        }
    }

    pub fn from_source<T: std::error::Error + 'static>(context: &str, source: T) -> Self {
        Self {
            context: context.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn context(&self) -> &str {
        &self.context
    }
}

impl std::fmt::Display for ProposalStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(ref source) = self.source {
            write!(
                f,
                "ProposalStoreError: Source: {} Context: {}",
                source, self.context
            )
        } else {
            write!(f, "ProposalStoreError: Context {}", self.context)
        }
    }
}
