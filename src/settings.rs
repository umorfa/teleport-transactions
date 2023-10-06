use config::{Config, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::app_data_dir;

static SETTINGS: OnceLock<Settings> = OnceLock::new();

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Settings {
    pub blockchain: BlockchainSettings,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BlockchainSettings {
    pub network: String,
    pub rpc_host: String,
    /// default ports: mainnet=8332, testnet=18332, regtest=18443, signet=38332
    pub rpc_port: u16,
    pub rpc_user: Option<String>,
    pub rpc_password: Option<String>,
    pub rpc_cookie_file: Option<String>,
    pub rpc_wallet_file: String,
}

impl Settings {
    pub fn global() -> &'static Settings {
        SETTINGS.get().as_ref().expect("Settings not initialized")
    }
    pub fn init_settings() -> &'static Settings {

        let datadir = app_data_dir("teleport");
        let config_location = datadir.join("teleport.conf");

        let s = Config::builder()
            .add_source(File::new(
                    config_location.to_str().unwrap(),
                    FileFormat::Toml,
                    ))
            .add_source(Environment::with_prefix("TELEPORT"))
            .build()
            .unwrap();

        let settings = s.try_deserialize().unwrap();
        SETTINGS.set(settings).unwrap();
        Settings::global()
    }
}
