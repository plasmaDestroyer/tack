use std::error::Error;
use std::os::unix::process::CommandExt;
use std::process::Stdio;

use crate::manifest::{get_manifest_path, load_manifest};
use crate::util::{get_share_dir, slugify};

pub fn open_app(name: &str) -> Result<(), Box<dyn Error>> {
    let share_dir = get_share_dir()?;
    let slug = slugify(name);
    let manifest_path = get_manifest_path(&share_dir);
    let entries = load_manifest(&manifest_path)?;

    let entry = entries
        .iter()
        .find(|e| e.slug == slug)
        .ok_or_else(|| format!("App '{}' is not installed.", name))?;

    println!("Opening {} ({})", entry.name, entry.url);

    unsafe {
        std::process::Command::new(&entry.browser)
            .arg(format!("--app={}", entry.url))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .pre_exec(|| {
                libc::setsid();
                Ok(())
            })
            .spawn()?;
    }

    Ok(())
}
