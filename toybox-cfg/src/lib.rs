
pub mod prelude {}

pub mod table;
use table::{Table, Value};

use std::path::{PathBuf};


/// Runtime representation of hierarchical key-value storage, intended for settings, command line config, etc.
#[derive(Debug, Clone, Default)]
pub struct Config {
	/// Config loaded and saved to disk.
	base: Table,

	/// Any config overrided by CLI args.
	arguments: Table,

	/// Config set during runtime that can be either committed to base or reverted.
	preview: Table,

	/// Combined config with overrides applied.
	// TODO(pat.m): this is basically a cache but maybe I don't need this
	resolved: Table,

	save_path: PathBuf,
}

impl Config {
	pub fn for_app_name(app_name: &str) -> anyhow::Result<Self> {
		let mut config = Self::default();

		config.save_path = config_path(app_name);
		if config.save_path.exists() {
			config.base = Table::from_file(&config.save_path)?;
		} else {
			// TODO(pat.m): defaults?
			config.base.save_to_file(&config.save_path)?;
		}

		config.arguments = Table::from_cli()?;

		// TODO(pat.m): resolve

		log::info!("Loaded config: {config:?}");

		Ok(config)
	}

	pub fn save(&self) -> anyhow::Result<()> {
		// TODO(pat.m): extra resolve? 
		self.base.save_to_file(&self.save_path)
	}

	pub fn commit(&mut self) {
		self.base.merge_from(&self.preview);
		self.arguments.remove_values_in(&self.preview);

		// TODO(pat.m): this may not be needed if preview config is automatically added to resolved
		self.preview = Table::new();
		self.resolved = Table::new();
	}

	pub fn revert(&mut self) {
		self.preview = Table::new();
		self.resolved = Table::new();
	}
}

impl Config {
	pub fn get_value(&self, key: &str) -> Option<&Value> {
		// if let Some(value) = self.resolved.get_value(key) {
		// 	return Some(value)
		// }

		if let Some(value) = self.preview.get_value(key) {
			// self.resolved.set_value(key, value.clone());
			return Some(value)
		}

		if let Some(value) = self.arguments.get_value(key) {
			// self.resolved.set_value(key, value.clone());
			return Some(value)
		}

		if let Some(value) = self.base.get_value(key) {
			// self.resolved.set_value(key, value.clone());
			return Some(value)
		}

		None
	}

	// pub fn get_value_or(&mut self, key: &str, default: impl Into<Value>) -> &Value {
	// }
}


pub fn config_path(app_name: &str) -> PathBuf {
	let mut dir = dirs::preference_dir() 
		.expect("Couldn't get preferences dir");

	dir.push("toybox");
	dir.push(app_name);
	dir.push("config.toml");

	dir
}