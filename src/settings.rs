use config::{Config, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::utils::bitcoin_data_dir;

static SETTINGS: OnceLock<Settings> = OnceLock::new();

/// Global settings
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Settings {
    pub blockchain: BlockchainSettings,
    pub datadir: Option<PathBuf>,
}

/// Settings relating to the bitcoin node
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
    /// Return the file path to the bitcoin RPC cookie file.
    /// Note that this file only exists if bitcoind is actively running
    pub fn rpc_cookie_path(&self) -> PathBuf {
        let cookie_file = match &self.rpc_cookie_file {
            Some(f) => f.as_str(),
            None => ".cookie",
        };
        bitcoin_data_dir(&self.network).join(cookie_file)
    }

    /// Return the RPC URL
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

    pub fn init_settings(datadir: &PathBuf) -> &'static Settings {
        let config_location = datadir.join("teleport.conf");

        let s = Config::builder()
            .add_source(Config::try_from(&Settings::default()).unwrap())
            .add_source(
                File::new(config_location.to_str().unwrap(), FileFormat::Toml).required(false),
            )
            .set_override("datadir", datadir.to_str())
            .unwrap()
            .build()
            .unwrap();

        let settings = s.try_deserialize().unwrap();
        SETTINGS.set(settings).unwrap();
        Settings::global()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            blockchain: BlockchainSettings {
                network: "regtest".to_string(),
                rpc_host: "localhost".to_string(),
                rpc_port: 18443,
                rpc_user: None,
                rpc_password: None,
                rpc_cookie_file: None,
                rpc_wallet_file: "teleport".to_string(),
            },
            datadir: None,
        }
    }
}
