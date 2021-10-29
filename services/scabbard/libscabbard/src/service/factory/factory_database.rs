// Copyright 2018-2021 Cargill Incorporated
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

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use cylinder::VerifierFactory;
#[cfg(any(feature = "postgres", feature = "sqlite"))]
use diesel::r2d2::{ConnectionManager, Pool};
#[cfg(any(feature = "postgres", feature = "sqlite"))]
use sawtooth::receipt::store::diesel::DieselReceiptStore;
use sawtooth::receipt::store::ReceiptStore;
use splinter::error::InvalidArgumentError;
use splinter::error::{InternalError, InvalidStateError};
use splinter::service::validation::ServiceArgValidator;
use splinter::service::{FactoryCreateError, Service, ServiceFactory};
use transact::database::Database;

use crate::hex::parse_hex;
#[cfg(feature = "rest-api-actix")]
use crate::service::rest_api::actix;
use crate::service::{
    error::ScabbardError,
    state::merkle_state::{MerkleState, MerkleStateConfig},
    Scabbard, ScabbardVersion, SERVICE_TYPE,
};
use crate::store::{
    diesel::DieselCommitHashStore, transact::factory::LmdbDatabaseFactory, CommitHashStore,
};

const DEFAULT_LMDB_DIR: &str = "/var/lib/splinter";

type ScabbardReceiptStore = Arc<RwLock<dyn ReceiptStore>>;

#[cfg(any(feature = "postgres", feature = "sqlite"))]
#[derive(Clone)]
pub enum ConnectionUri {
    #[cfg(feature = "postgres")]
    Postgres(Box<str>),
    #[cfg(feature = "sqlite")]
    Sqlite(Box<str>),
    #[cfg(not(feature = "sqlite"))]
    Unknown(Box<str>),
}

#[derive(Default)]
pub struct ScabbardLmdbStateConfiguration {
    db_dir: Option<String>,
    db_size: Option<usize>,
    enable_lmdb: bool,
}

/// Configuration for underlying storage that will be enabled for each service produced by the
/// resulting ScabbardFactory.
#[cfg(any(feature = "postgres", feature = "sqlite"))]
#[derive(Clone)]
pub enum ScabbardStorageConfiguration {
    ConnectionUri {
        connection_uri: ConnectionUri,
    },
    #[cfg(feature = "postgres")]
    Postgres {
        pool: Pool<ConnectionManager<diesel::pg::PgConnection>>,
    },
    #[cfg(feature = "sqlite")]
    Sqlite {
        pool: Pool<ConnectionManager<diesel::SqliteConnection>>,
    },
}

impl ScabbardStorageConfiguration {
    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    pub fn connection_uri(connection_uri: &str) -> Self {
        let connection_uri = match connection_uri {
            #[cfg(feature = "postgres")]
            s if s.starts_with("postgres://") => ConnectionUri::Postgres(s.into()),
            #[cfg(feature = "sqlite")]
            s => ConnectionUri::Sqlite(s.into()),
            #[cfg(not(feature = "sqlite"))]
            s => ConnectionUri::Unknown(s.into()),
        };
        Self::ConnectionUri { connection_uri }
    }
}

#[cfg(feature = "postgres")]
impl From<Pool<ConnectionManager<diesel::pg::PgConnection>>> for ScabbardStorageConfiguration {
    fn from(pool: Pool<ConnectionManager<diesel::pg::PgConnection>>) -> Self {
        Self::Postgres { pool }
    }
}

#[cfg(feature = "sqlite")]
impl From<Pool<ConnectionManager<diesel::SqliteConnection>>> for ScabbardStorageConfiguration {
    fn from(pool: Pool<ConnectionManager<diesel::SqliteConnection>>) -> Self {
        Self::Sqlite { pool }
    }
}

/// Builds new ScabbardFactory instances.
#[derive(Default)]
pub struct ScabbardFactoryBuilder {
    state_storage_configuration: Option<ScabbardLmdbStateConfiguration>,
    storage_configuration: Option<ScabbardStorageConfiguration>,
    signature_verifier_factory: Option<Arc<Mutex<Box<dyn VerifierFactory>>>>,
}

impl ScabbardFactoryBuilder {
    /// Constructs a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures the services to be constructed using LMDB for storing transaction state.
    pub fn with_lmdb_state_defaults(mut self) -> Self {
        self.state_storage_configuration = Some(ScabbardLmdbStateConfiguration::default());
        self
    }

    /// Sets the state db directory to be used by the resulting factory.
    pub fn with_lmdb_state_db_dir(mut self, state_db_dir: String) -> Self {
        self.state_storage_configuration = self
            .state_storage_configuration
            .take()
            .or_else(|| Some(ScabbardLmdbStateConfiguration::default()))
            .map(|mut config| {
                config.db_dir = Some(state_db_dir);
                config
            });
        self
    }

    /// Sets the state db size to be used by the resulting factory.
    pub fn with_lmdb_state_db_size(mut self, state_db_size: usize) -> Self {
        self.state_storage_configuration = self
            .state_storage_configuration
            .take()
            .or_else(|| Some(ScabbardLmdbStateConfiguration::default()))
            .map(|mut config| {
                config.db_size = Some(state_db_size);
                config
            });
        self
    }

    pub fn with_lmdb_state_enabled(mut self, enable: bool) -> Self {
        self.state_storage_configuration = self
            .state_storage_configuration
            .take()
            .or_else(|| Some(ScabbardLmdbStateConfiguration::default()))
            .map(|mut config| {
                config.enable_lmdb = enable;
                config
            });

        self
    }

    pub fn with_storage_configuration(
        mut self,
        storage_configuration: ScabbardStorageConfiguration,
    ) -> Self {
        self.storage_configuration = Some(storage_configuration);
        self
    }

    /// Set the signature verifier factory to be used by the resulting factory.  This is a required
    /// value, and omitting it will result in an [splinter::error::InvalidStateError] at build-time.
    pub fn with_signature_verifier_factory(
        mut self,
        signature_verifier_factory: Arc<Mutex<Box<dyn VerifierFactory>>>,
    ) -> Self {
        self.signature_verifier_factory = Some(signature_verifier_factory);
        self
    }

    /// Build the final [ScabbardFactory] instance.
    ///
    /// # Errors
    ///
    /// Returns an InvalidStateError if a signature_verifier_factory has not been set.
    pub fn build(self) -> Result<ScabbardFactory, InvalidStateError> {
        let signature_verifier_factory = self.signature_verifier_factory.ok_or_else(|| {
            splinter::error::InvalidStateError::with_message(
                "A scabbard factory requires a signature verifier factory".into(),
            )
        })?;

        let storage_configuration = self.storage_configuration.ok_or_else(|| {
            InvalidStateError::with_message("A storage configuration must be provided".into())
        })?;

        let state_storage_configuration = self.state_storage_configuration.unwrap_or_default();

        let state_store_factory = LmdbDatabaseFactory::new_state_db_factory(
            Path::new(
                state_storage_configuration
                    .db_dir
                    .as_deref()
                    .unwrap_or(DEFAULT_LMDB_DIR),
            ),
            state_storage_configuration.db_size,
        );

        let store_factory_config = match storage_configuration {
            #[cfg(feature = "postgres")]
            ScabbardStorageConfiguration::ConnectionUri {
                connection_uri: ConnectionUri::Postgres(url),
            } => ScabbardFactoryStorageConfig::Postgres {
                pool: get_postges_pool(&*url)?,
            },
            #[cfg(feature = "sqlite")]
            ScabbardStorageConfiguration::ConnectionUri {
                connection_uri: ConnectionUri::Sqlite(conn_str),
            } => ScabbardFactoryStorageConfig::Sqlite {
                pool: get_sqlite_pool(&*conn_str)?,
            },
            #[cfg(feature = "postgres")]
            ScabbardStorageConfiguration::Postgres { pool } => {
                ScabbardFactoryStorageConfig::Postgres { pool }
            }
            #[cfg(feature = "sqlite")]
            ScabbardStorageConfiguration::Sqlite { pool } => {
                ScabbardFactoryStorageConfig::Sqlite { pool }
            }
        };

        Ok(ScabbardFactory {
            service_types: vec![SERVICE_TYPE.into()],
            state_store_factory,
            enable_lmdb_state: state_storage_configuration.enable_lmdb,
            store_factory_config,
            signature_verifier_factory,
        })
    }
}
/// Internal Factory storage configuration.
enum ScabbardFactoryStorageConfig {
    #[cfg(feature = "postgres")]
    Postgres {
        pool: Pool<ConnectionManager<diesel::pg::PgConnection>>,
    },
    #[cfg(feature = "sqlite")]
    Sqlite {
        pool: Pool<ConnectionManager<diesel::SqliteConnection>>,
    },
}

pub struct ScabbardFactory {
    service_types: Vec<String>,
    state_store_factory: LmdbDatabaseFactory,
    enable_lmdb_state: bool,
    store_factory_config: ScabbardFactoryStorageConfig,
    signature_verifier_factory: Arc<Mutex<Box<dyn VerifierFactory>>>,
}

pub struct ScabbardArgValidator;

impl ServiceArgValidator for ScabbardArgValidator {
    fn validate(&self, args: &HashMap<String, String>) -> Result<(), InvalidArgumentError> {
        let peer_services_str = args.get("peer_services").ok_or_else(|| {
            InvalidArgumentError::new("peer_services".into(), "argument not provided".into())
        })?;

        let peer_services = parse_list(peer_services_str).map_err(|err| {
            InvalidArgumentError::new(
                "peer_services".into(),
                format!("failed to parse list: {}", err,),
            )
        })?;

        for service in peer_services {
            if service.is_empty() {
                return Err(InvalidArgumentError::new(
                    "peer_services".into(),
                    "must provide at least one service ID".into(),
                ));
            }
        }

        let admin_keys_str = args.get("admin_keys").ok_or_else(|| {
            InvalidArgumentError::new("admin_keys".into(), "argument not provided".into())
        })?;

        let admin_keys = parse_list(admin_keys_str).map_err(|err| {
            InvalidArgumentError::new(
                "admin_keys".into(),
                format!("failed to parse list: {}", err,),
            )
        })?;

        for key in admin_keys {
            if key.is_empty() {
                return Err(InvalidArgumentError::new(
                    "admin_keys".into(),
                    "must provide at least one admin key".into(),
                ));
            }

            let key_bytes = parse_hex(&key).map_err(|_| {
                InvalidArgumentError::new(
                    "admin_keys".into(),
                    format!("{:?} is not a valid hex-formatted public key", key,),
                )
            })?;

            if key_bytes.len() != 33 {
                return Err(InvalidArgumentError::new(
                    "admin_keys".into(),
                    format!("{} is not a valid public key: invalid length", key),
                ));
            }
        }

        Ok(())
    }
}

impl ServiceFactory for ScabbardFactory {
    fn available_service_types(&self) -> &[String] {
        self.service_types.as_slice()
    }

    /// `args` must include the following:
    /// - `admin_keys`: list of public keys that are allowed to create and modify sabre contracts,
    ///   formatted as a serialized JSON array of strings
    /// - `peer_services`: list of other scabbard services on the same circuit that this service
    ///   will share state with
    ///
    /// `args` may include the following optional entries:
    /// - `coordinator_timeout`: the length of time (in milliseconds) that the network has to
    ///   commit a proposal before the coordinator rejects it (if not provided, default is 30
    ///   seconds)
    /// - `version`: the protocol version for scabbard (possible values: "1", "2") (default: "1")
    fn create(
        &self,
        service_id: String,
        _service_type: &str,
        circuit_id: &str,
        args: HashMap<String, String>,
    ) -> Result<Box<dyn Service>, FactoryCreateError> {
        let peer_services_str = args.get("peer_services").ok_or_else(|| {
            FactoryCreateError::InvalidArguments("peer_services argument not provided".into())
        })?;

        let peer_services = parse_list(peer_services_str)
            .map_err(|err| {
                FactoryCreateError::InvalidArguments(format!(
                    "failed to parse peer_services list: {}",
                    err,
                ))
            })?
            .into_iter()
            .collect::<HashSet<String>>();

        let admin_keys_str = args.get("admin_keys").ok_or_else(|| {
            FactoryCreateError::InvalidArguments("admin_keys argument not provided".into())
        })?;

        let admin_keys = parse_list(admin_keys_str).map_err(|err| {
            FactoryCreateError::InvalidArguments(format!(
                "failed to parse admin_keys list: {}",
                err,
            ))
        })?;

        let coordinator_timeout = args
            .get("coordinator_timeout")
            .map(|timeout| match timeout.parse::<u64>() {
                Ok(timeout) => Ok(Duration::from_millis(timeout)),
                Err(err) => Err(FactoryCreateError::InvalidArguments(format!(
                    "invalid coordinator_timeout: {}",
                    err
                ))),
            })
            .transpose()?;
        let version = ScabbardVersion::try_from(args.get("version").map(String::as_str))
            .map_err(FactoryCreateError::InvalidArguments)?;

        let (merkle_state, state_purge) = if self.enable_lmdb_state {
            self.sql_state_check(circuit_id, &service_id)?;

            let db = self
                .state_store_factory
                .get_database(circuit_id, &service_id)
                .map_err(|e| FactoryCreateError::Internal(e.to_string()))?;

            let db_purge_handle = self
                .state_store_factory
                .get_database_purge_handle(circuit_id, &service_id)
                .map_err(|e| FactoryCreateError::Internal(e.to_string()))?;

            let merkle_state = MerkleState::new(MerkleStateConfig::key_value(db.clone_box()))
                .map_err(|e| FactoryCreateError::Internal(e.to_string()))?;

            (
                merkle_state,
                Box::new(move || {
                    db_purge_handle.purge()?;
                    Ok(())
                }) as Box<dyn Fn() -> Result<(), InternalError> + Sync + Send>,
            )
        } else {
            self.lmdb_state_check(circuit_id, &service_id)?;

            (
                MerkleState::new(self.create_sql_merkle_state_config(circuit_id, &service_id))
                    .map_err(|e| FactoryCreateError::Internal(e.to_string()))?,
                Box::new(|| Ok(())) as Box<dyn Fn() -> Result<(), InternalError> + Sync + Send>,
            )
        };

        let (receipt_store, commit_hash_store): (ScabbardReceiptStore, Box<dyn CommitHashStore>) =
            match &self.store_factory_config {
                #[cfg(feature = "postgres")]
                ScabbardFactoryStorageConfig::Postgres { pool } => (
                    Arc::new(RwLock::new(DieselReceiptStore::new(
                        pool.clone(),
                        Some(format!("{}::{}", circuit_id, service_id)),
                    ))),
                    Box::new(DieselCommitHashStore::new(
                        pool.clone(),
                        circuit_id,
                        &service_id,
                    )),
                ),
                #[cfg(feature = "sqlite")]
                ScabbardFactoryStorageConfig::Sqlite { pool } => (
                    Arc::new(RwLock::new(DieselReceiptStore::new(
                        pool.clone(),
                        Some(format!("{}::{}", circuit_id, service_id)),
                    ))),
                    Box::new(DieselCommitHashStore::new(
                        pool.clone(),
                        circuit_id,
                        &service_id,
                    )),
                ),
            };

        let purge_fn = state_purge as Box<dyn Fn() -> Result<(), InternalError> + Sync + Send>;

        let service = Scabbard::new(
            service_id,
            circuit_id,
            version,
            peer_services,
            merkle_state,
            commit_hash_store,
            receipt_store,
            purge_fn,
            self.signature_verifier_factory
                .lock()
                .map_err(|_| {
                    FactoryCreateError::CreationFailed(Box::new(ScabbardError::LockPoisoned))
                })?
                .new_verifier(),
            admin_keys,
            coordinator_timeout,
        )
        .map_err(|err| FactoryCreateError::CreationFailed(Box::new(err)))?;

        Ok(Box::new(service))
    }

    #[cfg(feature = "rest-api")]
    /// The `Scabbard` services created by the `ScabbardFactory` provide the following REST API
    /// endpoints as [`ServiceEndpoint`]s:
    ///
    /// * `POST /batches` - Add one or more batches to scabbard's queue
    /// * `GET /batch_statuses` - Get the status of one or more batches
    /// * `GET /ws/subscribe` - Subscribe to scabbard state-delta events
    /// * `GET /state/{address}` - Get a value from scabbard's state
    /// * `GET /state` - Get multiple scabbard state entries
    /// * `GET /state_root` - Get the current state root hash of scabbard's state
    ///
    /// These endpoints are only available if the following REST API backend feature is enabled:
    ///
    /// * `rest-api-actix`
    ///
    /// [`ServiceEndpoint`]: ../rest_api/struct.ServiceEndpoint.html
    fn get_rest_endpoints(&self) -> Vec<splinter::service::rest_api::ServiceEndpoint> {
        // Allowing unused_mut because resources must be mutable if feature rest-api-actix is
        // enabled
        #[allow(unused_mut)]
        let mut endpoints = vec![];

        #[cfg(feature = "rest-api-actix")]
        {
            endpoints.append(&mut vec![
                actix::batches::make_add_batches_to_queue_endpoint(),
                actix::ws_subscribe::make_subscribe_endpoint(),
                actix::batch_statuses::make_get_batch_status_endpoint(),
                actix::state_address::make_get_state_at_address_endpoint(),
                actix::state::make_get_state_with_prefix_endpoint(),
                actix::state_root::make_get_state_root_endpoint(),
            ])
        }

        endpoints
    }
}

impl ScabbardFactory {
    /// Check that the LMDB files doesn't exist for the given service.
    fn lmdb_state_check(
        &self,
        circuit_id: &str,
        service_id: &str,
    ) -> Result<(), FactoryCreateError> {
        let path = self
            .state_store_factory
            .compute_path(circuit_id, service_id)
            .map_err(|e| FactoryCreateError::Internal(e.to_string()))?;
        if path.with_extension("lmdb").exists() {
            return Err(InvalidStateError::with_message(format!(
                "LMDB database files exist for {}::{}, but LMDB storage is not enabled",
                circuit_id, service_id
            ))
            .into());
        }
        Ok(())
    }

    /// Check that the SQL state doesn't exist for the given service.
    fn sql_state_check(&self, circuit_id: &str, service_id: &str) -> Result<(), InvalidStateError> {
        let exists = MerkleState::check_existence(
            &self.create_sql_merkle_state_config(circuit_id, service_id),
        );

        if exists {
            return Err(InvalidStateError::with_message(format!(
                "A SQL-based merkle tree exists for {}::{}, but database storage is not enabled",
                circuit_id, service_id
            )));
        }

        Ok(())
    }

    fn create_sql_merkle_state_config(
        &self,
        circuit_id: &str,
        service_id: &str,
    ) -> MerkleStateConfig {
        match &self.store_factory_config {
            #[cfg(feature = "postgres")]
            ScabbardFactoryStorageConfig::Postgres { pool } => MerkleStateConfig::Postgres {
                pool: pool.clone(),
                tree_name: format!("{}::{}", circuit_id, service_id),
            },
            #[cfg(feature = "sqlite")]
            ScabbardFactoryStorageConfig::Sqlite { pool } => MerkleStateConfig::Sqlite {
                pool: pool.clone(),
                tree_name: format!("{}::{}", circuit_id, service_id),
            },
        }
    }
}

/// Parse a service argument into a list. Check if the argument is in json or csv format
/// and return the list of strings. An error is returned if json fmt cannot be parsed.
fn parse_list(values_list: &str) -> Result<Vec<String>, String> {
    if values_list.starts_with('[') {
        serde_json::from_str(values_list).map_err(|err| err.to_string())
    } else {
        Ok(values_list
            .split(',')
            .map(String::from)
            .collect::<Vec<String>>())
    }
}

#[cfg(feature = "postgres")]
fn get_postges_pool(
    url: &str,
) -> Result<Pool<ConnectionManager<diesel::pg::PgConnection>>, InvalidStateError> {
    let connection_manager = ConnectionManager::<diesel::pg::PgConnection>::new(url);
    Pool::builder().build(connection_manager).map_err(|err| {
        InvalidStateError::with_message(format!("Failed to build connection pool: {}", err))
    })
}

#[cfg(feature = "sqlite")]
fn get_sqlite_pool(
    conn_str: &str,
) -> Result<Pool<ConnectionManager<diesel::SqliteConnection>>, InvalidStateError> {
    if (&*conn_str != ":memory:") && !Path::new(&*conn_str).exists() {
        return Err(InvalidStateError::with_message(format!(
            "Database file '{}' does not exist",
            conn_str
        )));
    }
    let connection_manager = ConnectionManager::<diesel::sqlite::SqliteConnection>::new(&*conn_str);
    let mut pool_builder = Pool::builder();
    // A new database is created for each connection to the in-memory SQLite
    // implementation; to ensure that the resulting stores will operate on the same
    // database, only one connection is allowed.
    if &*conn_str == ":memory:" {
        pool_builder = pool_builder.max_size(1);
    }
    pool_builder.build(connection_manager).map_err(|err| {
        InvalidStateError::with_message(format!("Failed to build connection pool: {}", err))
    })
}

#[cfg(feature = "sqlite")]
#[cfg(test)]
mod tests {
    use super::*;

    use cylinder::{secp256k1::Secp256k1Context, Context};
    use transact::state::merkle::sql::{backend::SqliteBackend, migration::MigrationManager};

    /// Verify that the scabbard factory produces a valid `Scabbard` instance.
    #[test]
    fn create_successful() {
        let factory = get_factory();

        let service = factory
            .create("0".into(), "", "1", get_mock_args())
            .expect("failed to create service");
        assert_eq!(service.service_id(), "0");

        let scabbard = (&*service)
            .as_any()
            .downcast_ref::<Scabbard>()
            .expect("failed to downcast Service to Scabbard");
        assert_eq!(&scabbard.service_id, "0");
        assert_eq!(&scabbard.circuit_id, "1");
    }

    /// Verify that the scabbard factory produces a valid `Scabbard` instance if the service
    /// arguments are commo seperated instead of json fmt.
    #[test]
    fn create_successful_no_json() {
        let factory = get_factory();

        let service = factory
            .create("2".into(), "", "1", get_mock_args_no_json())
            .expect("failed to create service");
        assert_eq!(service.service_id(), "2");

        let scabbard = (&*service)
            .as_any()
            .downcast_ref::<Scabbard>()
            .expect("failed to downcast Service to Scabbard");
        assert_eq!(&scabbard.service_id, "2");
        assert_eq!(&scabbard.circuit_id, "1");
    }

    /// Verify that the `coordinator_timeout` service argument is properly set for a new `Scabbard`
    /// instance.
    #[test]
    fn create_with_coordinator_timeout() {
        let factory = get_factory();
        let mut args = get_mock_args();
        args.insert("coordinator_timeout".into(), "123".into());

        let service = factory
            .create("".into(), "", "", args)
            .expect("failed to create service");
        let scabbard = (&*service)
            .as_any()
            .downcast_ref::<Scabbard>()
            .expect("failed to downcast Service to Scabbard");

        assert_eq!(scabbard.coordinator_timeout, Duration::from_millis(123));
    }

    /// Verify that `Scabbard` creation fails when the `peer_services` argument isn't specified.
    #[test]
    fn create_without_peer_services() {
        let factory = get_factory();
        let mut args = get_mock_args();
        args.remove("peer_services");

        assert!(
            factory.create("".into(), "", "", args).is_err(),
            "Creating factory without peer_services did not fail"
        );
    }

    /// Verify that `Scabbard` creation fails when the `admin_keys` argument isn't specified.
    #[test]
    fn create_without_admin_keys() {
        let factory = get_factory();
        let mut args = get_mock_args();
        args.remove("admin_keys");

        assert!(
            factory.create("".into(), "", "", args).is_err(),
            "Creating factory without admin_keys did not fail"
        );
    }

    /// Verify arg validation returns ok with valid common seperated Args
    #[test]
    fn test_valid_argument_validation_no_json() {
        let validator = ScabbardArgValidator;
        let args = get_mock_args_no_json();
        assert!(validator.validate(&args).is_ok());
    }

    /// Verify arg validation returns ok with valid Args using json
    #[test]
    fn test_valid_argument_validation() {
        let validator = ScabbardArgValidator;
        let args = get_mock_args();
        assert!(validator.validate(&args).is_ok());
    }

    /// Verify arg validation returns an error if the args are empty
    #[test]
    fn test_no_argument_validation() {
        let validator = ScabbardArgValidator;
        let args = HashMap::new();
        assert!(validator.validate(&args).is_err());
    }

    /// Verify arg validation returns an error if the args are present but the values are empty
    #[test]
    fn test_empty_argument_validation() {
        let validator = ScabbardArgValidator;
        let mut args = HashMap::new();
        args.insert("peer_services".into(), "".into());
        args.insert("admin_keys".into(), "".into());
        assert!(validator.validate(&args).is_err());
    }

    fn get_factory() -> ScabbardFactory {
        let connection_manager = ConnectionManager::<diesel::SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1)
            .build(connection_manager)
            .expect("Failed to build connection pool");

        sawtooth::migrations::run_sqlite_migrations(
            &*pool.get().expect("Failed to get connection for migrations"),
        )
        .expect("Failed to run migrations");
        crate::migrations::run_sqlite_migrations(
            &*pool.get().expect("Failed to get connection for migrations"),
        )
        .expect("Failed to run migrations");
        let backend = SqliteBackend::from(pool.clone());
        backend
            .run_migrations()
            .expect("Failed to run transact migrations");

        let store_factory_config = ScabbardFactoryStorageConfig::Sqlite { pool };
        ScabbardFactory {
            service_types: vec![SERVICE_TYPE.into()],
            state_store_factory: LmdbDatabaseFactory::new_state_db_factory(
                &Path::new("/tmp"),
                None,
            ),
            enable_lmdb_state: false,
            store_factory_config,
            signature_verifier_factory: Arc::new(Mutex::new(Box::new(Secp256k1Context::new()))),
        }
    }

    fn get_mock_args() -> HashMap<String, String> {
        let peer_services = vec!["1".to_string(), "2".to_string(), "3".to_string()];
        let admin_keys = vec![get_public_key(), get_public_key()];
        let mut args = HashMap::new();
        args.insert(
            "peer_services".into(),
            serde_json::to_string(&peer_services).expect("failed to serialize peer_services"),
        );
        args.insert(
            "admin_keys".into(),
            serde_json::to_string(&admin_keys).expect("failed to serialize admin_keys"),
        );
        args
    }

    fn get_mock_args_no_json() -> HashMap<String, String> {
        let mut args = HashMap::new();
        args.insert("peer_services".into(), "0,1,3".into());
        args.insert(
            "admin_keys".into(),
            format!("{},{}", get_public_key(), get_public_key()),
        );
        args
    }

    fn get_public_key() -> String {
        let context = Secp256k1Context::new();
        let private_key = context.new_random_private_key();
        let public_key = context.get_public_key(&private_key).unwrap();
        public_key.as_hex()
    }
}