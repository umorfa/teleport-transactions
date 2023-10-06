use dirs::{data_dir, home_dir};
use std::path::PathBuf;

fn make_ascii_titlecase(s: &mut str) {
    if let Some(r) = s.get_mut(0..1) {
        r.make_ascii_uppercase();
    }
}

/// Return the data directory for the user's platform
///
/// | Platform | Value                                         |
/// | -------- | --------------------------------------------- |
/// | Linux    | `$HOME/.teleport/`                            |
/// | macOS    | `$HOME/Library/Application Support/Teleport/` |
/// | Windows  | `%APPDATA%\Teleport\`                         |
pub fn app_data_dir(appname: &str) -> PathBuf {
    let mut appname = appname.trim().to_lowercase();
    if cfg!(any(target_os = "macos", target_os = "windows")) {
        make_ascii_titlecase(&mut appname);
        return data_dir().unwrap().join(appname);
    }
    home_dir().unwrap().join(format!(".{}", appname))
}

