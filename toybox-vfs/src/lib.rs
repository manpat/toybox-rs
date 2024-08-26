#![feature(let_chains)]

use std::path::{Path, PathBuf};
use anyhow::Context;

pub mod prelude {}


pub struct Vfs {
    resource_root: Box<Path>,
}

impl Vfs {
    pub fn new() -> anyhow::Result<Vfs> {
        Ok(Vfs {
            resource_root: find_resource_folder()
                .context("Can't find resource directory")?
                .into_boxed_path()
        })
    }

    pub fn resource_root(&self) -> &Path {
        &self.resource_root
    }

    pub fn resource_path(&self, virtual_path: impl AsRef<Path>) -> PathBuf {
        let components = virtual_path.as_ref().components();

        let clean_path = clean_virtual_path(components)
            .with_context(|| virtual_path.as_ref().display().to_string())
            .expect("Failed to clean resource path");

        self.resource_root.join(clean_path)
    }
}



fn clean_virtual_path(mut components: std::path::Components<'_>) -> anyhow::Result<&Path> {
    use std::path::Component;

    for component in components.clone() {
        match component {
            Component::RootDir => {
                // Strip root prefix - virtual paths are always relative to resource root
                let _ = components.next();
            }

            Component::Normal(_) | Component::CurDir => {}

            Component::ParentDir => anyhow::bail!("References to parent directories '..' in resource paths are not allowed."),
            Component::Prefix(prefix) => anyhow::bail!("Path prefixes (like {:?}) in resource paths are not allowed.", prefix.as_os_str()),
        }
    }

    Ok(components.as_path())
}



fn find_resource_folder() -> anyhow::Result<PathBuf> {
    let mut dirs_scanned = Vec::new();

    // Scan from working directory
    if let Ok(Some(path)) = try_find_resource_folder_from(&std::env::current_dir()?, &mut dirs_scanned) {
        return Ok(path)
    }

    // Scan from executable path
    let exe_path = std::env::current_exe()?;

    let search_dir = exe_path.parent()
        .ok_or_else(|| anyhow::format_err!("Executable path invalid '{}'", exe_path.display()))?;

    if let Ok(Some(path)) = try_find_resource_folder_from(&search_dir, &mut dirs_scanned) {
        return Ok(path)
    }

    anyhow::bail!("Couldn't find 'resource' folder in any directory above the executable path or working directory.\nScanned directories: {:?}", dirs_scanned)
}

fn try_find_resource_folder_from(search_dir: &Path, dirs_scanned: &mut Vec<PathBuf>) -> anyhow::Result<Option<PathBuf>> {
    // Try scanning the current search dir first, and then one directory above.
    for search_dir in search_dir.ancestors().take(2) {
        // println!("Trying to scan {}", search_dir.display());

        let Ok(children) = search_dir.read_dir() else {
            continue
        };

        let mut to_scan = Vec::new();

        for dir_entry in children {
            if let Ok(dir_entry) = dir_entry
                && let Ok(file_type) = dir_entry.file_type()
                && file_type.is_dir()
            {
                let dir_path = dir_entry.path();

                // println!("=== Testing {}", dir_path.display());
                if dir_path.ends_with("resource") {
                    return Ok(Some(dir_path))
                }

                to_scan.push(dir_path);
            }
        }

        // If there are no resource folders in the search_dir, try one level deeper
        for dir_path in to_scan {
            if let Some(path) = try_find_resource_folder_in(&dir_path, dirs_scanned)? {
                return Ok(Some(path))
            }
        }
    }

    Ok(None)
}

fn try_find_resource_folder_in(search_dir: &Path, dirs_scanned: &mut Vec<PathBuf>) -> anyhow::Result<Option<PathBuf>> {
    let path = search_dir.join("resource");
    dirs_scanned.push(path.clone());
    // println!("=== Testing {}", path.display());

    if path.try_exists()? {
        return Ok(Some(path))
    }

    Ok(None)
}