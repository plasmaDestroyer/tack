use std::error::Error;
use std::fs;

use crate::manifest::{get_manifest_path, load_manifest};
use crate::output;
use crate::util::get_share_dir;

pub fn export_apps(output_path: Option<&str>) -> Result<(), Box<dyn Error>> {
    let share_dir = get_share_dir()?;
    let manifest_path = get_manifest_path(&share_dir);
    let entries = load_manifest(&manifest_path)?;

    let json = serde_json::to_string_pretty(&entries)?;

    if let Some(path) = output_path {
        fs::write(path, &json)?;
        output::success(&format!("Exported manifest to {}", path));
    } else {
        println!("{}", json);
    }

    Ok(())
}
