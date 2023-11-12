use std::path::{Path, PathBuf};



pub fn find_resource_folder() -> anyhow::Result<PathBuf> {
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