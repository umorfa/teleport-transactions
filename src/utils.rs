use crate::settings::Settings;
use dirs::{data_dir, home_dir};
use std::path::PathBuf;

fn make_ascii_titlecase(s: &mut str) {
    if let Some(r) = s.get_mut(0..1) {
        r.make_ascii_uppercase();
    }
}

/// Return the default data directory for the user's platform
///
/// | Platform | Value                                         |
/// | -------- | --------------------------------------------- |
/// | Linux    | `$HOME/.teleport/`                            |
/// | macOS    | `$HOME/Library/Application Support/Teleport/` |
/// | Windows  | `%APPDATA%\Teleport\`                         |
pub fn default_data_dir(appname: &str) -> PathBuf {
    let mut appname = appname.trim().to_lowercase();
    if cfg!(any(target_os = "macos", target_os = "windows")) {
        make_ascii_titlecase(&mut appname);
        return data_dir().unwrap().join(appname);
    }
    home_dir().unwrap().join(format!(".{}", appname))
}

/// Return the default data directory for Teleport, or the custom datadir if one was provided
pub fn teleport_data_dir() -> PathBuf {
    match &Settings::global().datadir {
        Some(d) => d.clone(),
        None => default_data_dir("teleport"),
    }
}

/// Return the network-specific bitcoin data directory
/// https://github.com/bitcoin/bitcoin/blob/master/doc/files.md#data-directory-location
pub fn bitcoin_data_dir(network: &str) -> PathBuf {
    let bitcoin_dir = default_data_dir("bitcoin");
    let network_subdir = match network {
        "main" => "",
        "testnet" => "testnet3",
        _ => network,
    };
    bitcoin_dir.join(network_subdir)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bitcoin_data_dir() {
        let main_dir = bitcoin_data_dir("main");
        let testnet_dir = bitcoin_data_dir("testnet");
        let signet_dir = bitcoin_data_dir("signet");
        let regtest_dir = bitcoin_data_dir("regtest");

        assert_eq!(main_dir, default_data_dir("bitcoin"));
        assert_eq!(testnet_dir, default_data_dir("bitcoin").join("testnet3"));
        assert_eq!(signet_dir, default_data_dir("bitcoin").join("signet"));
        assert_eq!(regtest_dir, default_data_dir("bitcoin").join("regtest"));
    }
}
