// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::{
	path::PathBuf,
	time::{SystemTime, UNIX_EPOCH},
};

use serde::{Serialize, ser::Serializer};
use tauri::{
	Runtime,
	command,
	plugin::{Builder as PluginBuilder, TauriPlugin},
};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error(transparent)]
	Io(#[from] std::io::Error),
}

impl Serialize for Error {
	fn serialize<S>(&self, serializer:S) -> std::result::Result<S::Ok, S::Error>
	where
		S: Serializer, {
		serializer.serialize_str(self.to_string().as_ref())
	}
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Permissions {
	readonly:bool,
	#[cfg(unix)]
	mode:u32,
}

#[cfg(unix)]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UnixMetadata {
	dev:u64,
	ino:u64,
	mode:u32,
	nlink:u64,
	uid:u32,
	gid:u32,
	rdev:u64,
	blksize:u64,
	blocks:u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Metadata {
	accessed_at_ms:u64,
	created_at_ms:u64,
	modified_at_ms:u64,
	is_dir:bool,
	is_file:bool,
	is_symlink:bool,
	size:u64,
	permissions:Permissions,
	#[cfg(unix)]
	#[serde(flatten)]
	unix:UnixMetadata,
	#[cfg(windows)]
	file_attributes:u32,
}

fn system_time_to_ms(time:std::io::Result<SystemTime>) -> u64 {
	time.map(|time| {
		time.duration_since(UNIX_EPOCH)
			.map(|t| t.as_millis() as u64)
			.unwrap_or_else(|err| err.duration().as_millis() as u64)
	})
	.unwrap_or_default()
}

#[command]
async fn metadata(path:PathBuf) -> Result<Metadata> {
	let metadata = std::fs::metadata(path)?;

	let file_type = metadata.file_type();

	let permissions = metadata.permissions();

	Ok(Metadata {
		accessed_at_ms:system_time_to_ms(metadata.accessed()),
		created_at_ms:system_time_to_ms(metadata.created()),
		modified_at_ms:system_time_to_ms(metadata.modified()),
		is_dir:file_type.is_dir(),
		is_file:file_type.is_file(),
		is_symlink:file_type.is_symlink(),
		size:metadata.len(),
		permissions:Permissions {
			readonly:permissions.readonly(),
			#[cfg(unix)]
			mode:permissions.mode(),
		},
		#[cfg(unix)]
		unix:UnixMetadata {
			dev:metadata.dev(),
			ino:metadata.ino(),
			mode:metadata.mode(),
			nlink:metadata.nlink(),
			uid:metadata.uid(),
			gid:metadata.gid(),
			rdev:metadata.rdev(),
			blksize:metadata.blksize(),
			blocks:metadata.blocks(),
		},
		#[cfg(windows)]
		file_attributes:metadata.file_attributes(),
	})
}

#[command]
async fn exists(path:PathBuf) -> bool { path.exists() }

pub fn init<R:Runtime>() -> TauriPlugin<R> {
	PluginBuilder::new("fs-extra")
		.invoke_handler(tauri::generate_handler![exists, metadata])
		.build()
}
