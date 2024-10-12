use std::collections::HashMap;
use std::path::Path;

use toybox_vfs::{Vfs, PathKind};


#[derive(Debug, Clone, Default)]
pub struct Table {
    data: HashMap<String, Value>,
}


impl Table {
	pub fn new() -> Table {
		Table::default()
	}

	pub fn from_file(vfs: &Vfs, kind: PathKind, path: impl AsRef<Path>) -> anyhow::Result<Table> {
		let data = vfs.load_string(kind, path)?;
		let raw: toml::Table = toml::from_str(&data)?;
		let _ = dbg!(raw);

		Ok(Table::default())
	}

	pub fn from_cli() -> anyhow::Result<Table> {
		let mut args = std::env::args();
		let _ = args.next(); // skip first arg

		let _ = dbg!(args);

		Ok(Table::default())
	}

	pub fn save_to_file(&self, vfs: &Vfs, kind: PathKind, path: impl AsRef<Path>) -> anyhow::Result<()> {
		let toml = self.to_toml();
		let string = toml::to_string_pretty(&toml)?;
		vfs.save_data(kind, path, &string)
	}

	/// Copy or replace values present in `other`
	pub fn merge_from(&mut self, _other: &Table) {
		todo!()
	}

	/// Recursively remove values from this table that are present in `other`
	pub fn remove_values_in(&mut self, _other: &Table) {
		todo!()
	}

	pub fn get_value(&self, key: &str) -> Option<&Value> {
		if let Some((key, tail)) = key.split_once('.') {
			let subtable = self.data.get(key)?
				.as_table()?;

			subtable.get_value(tail)
		} else {
			self.data.get(key)
		}
	}

	pub fn set_value(&mut self, key: &str, value: Value) {
		if let Some((key, tail)) = key.split_once('.') {
			let subtable = self.data.entry(key.into())
				.or_insert(Value::Table(Default::default()))
				.as_table_mut()
				.expect("Trying to add value to non-table value");

			subtable.set_value(tail, value);

		} else {
			self.data.insert(key.into(), value);
		}
	}

	fn to_toml(&self) -> toml::Table {
		let mut tbl = toml::Table::new();

		for (key, value) in self.data.iter() {
			let value = match value {
				Value::String(string) => toml::Value::String(string.clone()),
				Value::Table(table) => toml::Value::Table(table.to_toml()),
				Value::Bool(b) => toml::Value::Boolean(*b),
			};

			tbl.insert(key.clone(), value);
		}

		tbl
	}
}




#[derive(Debug, Clone)]
pub enum Value {
	String(String),
	Table(Table),
	Bool(bool),
}

impl Value {
	pub fn as_table(&self) -> Option<&Table> {
		match self {
			Value::Table(tbl) => Some(tbl),
			_ => None
		}
	}
	
	pub fn as_table_mut(&mut self) -> Option<&mut Table> {
		match self {
			Value::Table(tbl) => Some(tbl),
			_ => None
		}
	}
}