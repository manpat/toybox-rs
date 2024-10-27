
pub mod prelude {}

mod table;
use toml::{Table, Value};

use tracing::instrument;

use toybox_vfs::{Vfs, PathKind};

// TODO(pat.m): this should maybe become a _system_ rather than a normal object


/// Runtime representation of hierarchical key-value storage, intended for settings, command line config, etc.
#[derive(Debug, Clone, Default)]
pub struct Config {
	/// Config loaded and saved to disk.
	base: Table,

	/// Any config overriden by CLI args.
	arguments: Table,

	/// Config set during runtime that can be either committed to base or reverted.
	preview: Table,

	/// Combined config with overrides applied.
	// TODO(pat.m): this is basically a cache but maybe I don't need this
	resolved: Table,
}

impl Config {
	// TODO(pat.m): this whole function is jank as shit
	#[instrument(skip_all, name="cfg Config::from_vfs")]
	pub fn from_vfs(vfs: &Vfs) -> anyhow::Result<Self> {
		let mut config = Self::default();

		if vfs.path_exists(PathKind::Config, "config.toml") {
			config.base = table::load_from_vfs(vfs, PathKind::Config, "config.toml")?;

		} else {
			log::info!("Couldn't load config - writing defaults to '{}'", vfs.user_data_root().display());
			// TODO(pat.m): defaults?
			table::save_to_vfs(&config.base, vfs, PathKind::Config, "config.toml")?;
		}

		config.arguments = table::load_from_cli()?;

		// TODO(pat.m): resolve

		log::info!("Loaded config: {config:?}");

		Ok(config)
	}

	#[instrument(skip_all, name="cfg Config::save")]
	pub fn save(&self, vfs: &Vfs) -> anyhow::Result<()> {
		// TODO(pat.m): extra resolve? 
		table::save_to_vfs(&self.base, vfs, PathKind::Config, "config.toml")
	}

	#[instrument(skip_all, name="cfg Config::commit")]
	pub fn commit(&mut self) {
		// self.base.merge_from(&self.preview);
		// self.arguments.remove_values_in(&self.preview);

		// TODO(pat.m): this may not be needed if preview config is automatically added to resolved
		self.preview = Table::new();
		self.resolved = Table::new();

		todo!();
	}

	#[instrument(skip_all, name="cfg Config::revert")]
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

		if let Some(value) = table::get_value(&self.preview, key) {
			// self.resolved.set_value(key, value.clone());
			return Some(value)
		}

		if let Some(value) = table::get_value(&self.arguments, key) {
			// self.resolved.set_value(key, value.clone());
			return Some(value)
		}

		if let Some(value) = table::get_value(&self.base, key) {
			// self.resolved.set_value(key, value.clone());
			return Some(value)
		}

		None
	}

	pub fn get_bool(&self, key: &str) -> Option<bool> {
		self.get_value(key)
			.and_then(Value::as_bool)
	}

	pub fn get_string(&self, key: &str) -> Option<&str> {
		self.get_value(key)
			.and_then(Value::as_str)
	}

	// pub fn get_value_or(&mut self, key: &str, default: impl Into<Value>) -> &Value {
	// }
}

