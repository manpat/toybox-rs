#![feature(let_chains)]

use std::path::{Path, PathBuf};
use anyhow::Context;
use tracing::instrument;

pub mod prelude {}


#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum PathKind {
	Resource,
	UserData,

	Config,
}


pub struct Vfs {
	// Game data - immutable in release, editable by editors
	resource_root: Box<Path>,

	// All inter-session data - game saves, user config, etc
	user_data_root: Box<Path>,
}

impl Vfs {
	#[instrument(name="vfs init")]
	pub fn new(app_name: &str) -> anyhow::Result<Vfs> {
		let resource_root = find_resource_folder()
			.context("Can't find resource directory")?
			.into_boxed_path();

		let mut user_data_root = dirs::data_dir()
			.context("Can't find local data directory")?;

		user_data_root.push("toybox");
		user_data_root.push(app_name);

		let user_data_root = user_data_root.into_boxed_path();

		log::info!("Resource Root Path: {}", resource_root.display());
		log::info!("Data Root Path: {}", user_data_root.display());

		Ok(Vfs { resource_root, user_data_root })
	}

	pub fn resource_root(&self) -> &Path {
		&self.resource_root
	}

	pub fn user_data_root(&self) -> &Path {
		&self.user_data_root
	}

	fn resolve_root(&self, kind: PathKind) -> &Path {
		match kind {
			PathKind::Resource => &self.resource_root,
			PathKind::UserData => &self.user_data_root,
			PathKind::Config => &self.user_data_root,
		}
	}

	pub fn resolve_path(&self, kind: PathKind, virtual_path: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
		let components = virtual_path.as_ref().components();

		let clean_path = clean_virtual_path(components)
			.with_context(|| format!("Invalid path '{}'", virtual_path.as_ref().display()))?;

		Ok(self.resolve_root(kind).join(clean_path))
	}

	pub fn path_exists(&self, kind: PathKind, virtual_path: impl AsRef<Path>) -> bool {
		// TODO(pat.m): sketchy as hell for actual FS operations - but we'll leave it for now
		match self.resolve_path(kind, virtual_path) {
			Ok(path) => path.exists(),
			Err(_) => false,
		}
	}

	#[instrument(skip_all)]
	pub fn load_data(&self, kind: PathKind, virtual_path: impl AsRef<Path>) -> anyhow::Result<Vec<u8>> {
		let path = self.resolve_path(kind, virtual_path)?;
		std::fs::read(&path).map_err(Into::into)
	}

	#[instrument(skip_all)]
	pub fn load_string(&self, kind: PathKind, virtual_path: impl AsRef<Path>) -> anyhow::Result<String> {
		let path = self.resolve_path(kind, virtual_path)?;
		std::fs::read_to_string(&path).map_err(Into::into)
	}

	#[instrument(skip_all)]
	pub fn save_data(&self, kind: PathKind, virtual_path: impl AsRef<Path>, data: impl AsRef<[u8]>) -> anyhow::Result<()> {
		// TODO(pat.m): assert path kind is writable
		let path = self.resolve_path(kind, virtual_path)?;

		if let Some(parent_path) = path.parent() {
			std::fs::create_dir_all(parent_path)?;
		}

		std::fs::write(path, data).map_err(Into::into)
	}


	#[instrument(skip_all)]
	pub fn load_resource_data(&self, virtual_path: impl AsRef<Path>) -> anyhow::Result<Vec<u8>> {
		self.load_data(PathKind::Resource, virtual_path)
	}

	#[instrument(skip_all)]
	pub fn save_resource_data(&self, virtual_path: impl AsRef<Path>, data: impl AsRef<[u8]>) -> anyhow::Result<()> {
		self.save_data(PathKind::Resource, virtual_path, data)
	}

	#[instrument(skip_all)]
	pub fn load_json_resource<T>(&self, virtual_path: impl AsRef<Path>) -> anyhow::Result<T>
		where T: for<'a> serde::Deserialize<'a>
	{
		let data = self.load_string(PathKind::Resource, virtual_path)?;
		serde_json::from_str(&data).map_err(Into::into)
	}

	#[instrument(skip_all)]
	pub fn save_json_resource<T>(&self, virtual_path: impl AsRef<Path>, data: &T) -> anyhow::Result<()>
		where T: serde::Serialize
	{
		let data = match cfg!(debug_assertions) {
			true => serde_json::to_vec_pretty(data)?,
			false => serde_json::to_vec(data)?,
		};

		self.save_resource_data(virtual_path, &data)
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

			Component::CurDir => {}

			Component::Normal(text) => {
				for byte in text.as_encoded_bytes() {
					let is_valid = byte.is_ascii_alphanumeric()
						|| [b' ', b'_', b'-', b'.'].contains(&byte);

					anyhow::ensure!(is_valid, "Resource paths may only contain ascii alphanumeric characters or limited punctuation.");
				}
			}

			Component::ParentDir => anyhow::bail!("References to parent directories '..' in resource paths are not allowed."),
			Component::Prefix(prefix) => anyhow::bail!("Path prefixes (like {:?}) in resource paths are not allowed.", prefix.as_os_str()),
		}
	}

	Ok(components.as_path())
}



#[instrument]
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

#[instrument]
fn try_find_resource_folder_from(search_dir: &Path, dirs_scanned: &mut Vec<PathBuf>) -> anyhow::Result<Option<PathBuf>> {
	// Try scanning the current search dir first, and then one directory above.
	for search_dir in search_dir.ancestors().take(2) {
		log::trace!("Trying to scan {}", search_dir.display());

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

				log::trace!("=== Testing {}", dir_path.display());
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
	log::trace!("=== Testing {}", path.display());

	if path.try_exists()? {
		return Ok(Some(path))
	}

	Ok(None)
}

