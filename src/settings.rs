use config::{Config, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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

impl BlockchainSettings {
    pub fn rpc_cookie_path(&self) -> PathBuf {
        let bitcoin_dir = app_data_dir("bitcoin");
        let network_dir = match self.network.as_str() {
            "main" => "",
            "testnet" => "testnet3",
            _ => self.network.as_str(),
        };
        let cookie_file = match &self.rpc_cookie_file {
            Some(f) => f.as_str(),
            None => ".cookie",
        };
        bitcoin_dir.join(network_dir).join(cookie_file)
    }

    pub fn rpc_url(&self) -> String {
        format!(
            "http://{}:{}/wallet/{}",
            self.rpc_host, self.rpc_port, &self.rpc_wallet_file
        )
    }
}

impl Settings {
    pub fn global() -> &'static Settings {
        SETTINGS.get().as_ref().expect("Settings not initialized")
    }

    pub fn init_settings() -> &'static Settings {
        let datadir = app_data_dir("teleport");
        let config_location = datadir.join("teleport.conf");

        let s = Config::builder()
            .add_source(
                File::new(config_location.to_str().unwrap(), FileFormat::Toml).required(false),
            )
            .build()
            .unwrap();

        let settings = s.try_deserialize().unwrap();
        SETTINGS.set(settings).unwrap();
        Settings::global()
    }
}
