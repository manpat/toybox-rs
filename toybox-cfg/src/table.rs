use std::path::Path;

use toml::{Table, Value};
use toybox_vfs::{Vfs, PathKind};

// 	/// Copy or replace values present in `other`
// 	pub fn merge_from(&mut self, _other: &Table) {
// 		todo!()
// 	}

// 	/// Recursively remove values from this table that are present in `other`
// 	pub fn remove_values_in(&mut self, _other: &Table) {
// 		todo!()
// 	}



pub fn load_from_vfs(vfs: &Vfs, kind: PathKind, path: impl AsRef<Path>) -> anyhow::Result<Table> {
	let data = vfs.load_string(kind, path)?;
	toml::from_str(&data).map_err(Into::into)
}

pub fn load_from_cli() -> anyhow::Result<Table> {
	let mut args = std::env::args();
	let _ = args.next(); // skip first arg

	let mut table = Table::default();

	for arg in args {
		let Some((key, value_str)) = arg.split_once('=') else {
			log::warn!("Failed to parse CLI argument: '{arg}'");
			continue
		};

		set_value(&mut table, key.trim(), Value::String(value_str.trim().into()));
	}

	Ok(table)
}

pub fn save_to_vfs(table: &Table, vfs: &Vfs, kind: PathKind, path: impl AsRef<Path>) -> anyhow::Result<()> {
	let string = toml::to_string_pretty(table)?;
	vfs.save_data(kind, path, &string)
}

pub fn get_value<'t>(table: &'t Table, key: &str) -> Option<&'t Value> {
	if let Some((key, tail)) = key.split_once('.') {
		let subtable = table.get(key)?
			.as_table()?;

		get_value(subtable, tail)

	} else {
		table.get(key)
	}
}

pub fn set_value(table: &mut Table, key: &str, value: Value) {
	if let Some((key, tail)) = key.split_once('.') {
		let subtable = table.entry(key)
			.or_insert(Value::Table(Default::default()))
			.as_table_mut()
			.expect("Trying to add value to non-table value");

		set_value(subtable, tail, value);

	} else {
		table.insert(key.into(), value);
	}
}