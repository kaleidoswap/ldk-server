// This file is Copyright its original authors, visible in version control
// history.
//
// This file is licensed under the Apache License, Version 2.0 <LICENSE-APACHE
// or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// You may not use this file except in accordance with one or both of these
// licenses.

use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::str::FromStr;

use ldk_node::bip39::{Mnemonic, WordCount};
use ldk_node::entropy::NodeEntropy;
use log::info;

use crate::util::{create_dir_all_private, read_to_string_with_limit, write_new};

const DEFAULT_MNEMONIC_FILE: &str = "keys_mnemonic";
const MNEMONIC_FILE_SIZE_LIMIT: usize = 1024;
// Raw seed written by ldk-server releases that predate mnemonic entropy.
const LEGACY_SEED_FILE: &str = "keys_seed";
const LEGACY_SEED_LEN: usize = 64;

pub(crate) fn load_or_generate_node_entropy(storage_dir: &Path) -> io::Result<NodeEntropy> {
	let mnemonic_path = storage_dir.join(DEFAULT_MNEMONIC_FILE);

	let mnemonic = match read_to_string_with_limit(&mnemonic_path, MNEMONIC_FILE_SIZE_LIMIT) {
		Ok(raw) => Mnemonic::from_str(raw.trim()).map_err(|e| {
			io::Error::new(
				io::ErrorKind::InvalidData,
				format!("Invalid BIP39 mnemonic in {}: {}", mnemonic_path.display(), e),
			)
		})?,
		Err(e) if e.kind() == io::ErrorKind::NotFound => {
			if let Some(entropy) = load_legacy_seed(storage_dir)? {
				return Ok(entropy);
			}
			if let Some(parent) = mnemonic_path.parent() {
				create_dir_all_private(parent)?;
			}
			let mnemonic = Mnemonic::generate(WordCount::Words24).map_err(io::Error::other)?;
			write_new(&mnemonic_path, format!("{}\n", mnemonic).as_bytes(), 0o600)?;
			info!(
				"Generated new BIP39 mnemonic at {}. Back up this file securely — it is required to recover on-chain funds.",
				mnemonic_path.display()
			);
			mnemonic
		},
		Err(e) => return Err(e),
	};

	Ok(NodeEntropy::from_bip39_mnemonic(mnemonic, None))
}

fn load_legacy_seed(storage_dir: &Path) -> io::Result<Option<NodeEntropy>> {
	let seed_path = storage_dir.join(LEGACY_SEED_FILE);
	let file = match fs::File::open(&seed_path) {
		Ok(file) => file,
		Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
		Err(e) => return Err(e),
	};
	let mut bytes = Vec::with_capacity(LEGACY_SEED_LEN);
	file.take(LEGACY_SEED_LEN as u64 + 1).read_to_end(&mut bytes)?;
	let seed: [u8; LEGACY_SEED_LEN] = bytes.try_into().map_err(|_| {
		io::Error::new(
			io::ErrorKind::InvalidData,
			format!("{} is not a {LEGACY_SEED_LEN}-byte seed", seed_path.display()),
		)
	})?;
	info!("Using legacy seed file {} as node entropy", seed_path.display());
	Ok(Some(NodeEntropy::from_seed_bytes(seed)))
}

#[cfg(test)]
mod tests {
	use std::fs;
	use std::os::unix::fs::{MetadataExt, PermissionsExt};
	use std::path::PathBuf;

	use super::*;

	const KNOWN_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

	fn tempdir(tag: &str) -> PathBuf {
		let dir = std::env::temp_dir().join(format!(
			"ldk-server-entropy-test-{}-{}",
			tag,
			std::process::id()
		));
		let _ = fs::remove_dir_all(&dir);
		fs::create_dir_all(&dir).unwrap();
		dir
	}

	#[test]
	fn generates_mnemonic_on_fresh_start() {
		let dir = tempdir("fresh");

		load_or_generate_node_entropy(&dir).unwrap();

		let mnemonic_path = dir.join(DEFAULT_MNEMONIC_FILE);
		assert!(mnemonic_path.exists(), "keys_mnemonic was not created");

		let perms = fs::metadata(&mnemonic_path).unwrap().permissions();
		assert_eq!(perms.mode() & 0o777, 0o600, "expected 0600 permissions");

		let content = fs::read_to_string(&mnemonic_path).unwrap();
		let word_count = content.trim().split_whitespace().count();
		assert_eq!(word_count, 24, "expected 24-word mnemonic, got {}", word_count);

		let mtime_before = fs::metadata(&mnemonic_path).unwrap().mtime();
		load_or_generate_node_entropy(&dir).unwrap();
		let mtime_after = fs::metadata(&mnemonic_path).unwrap().mtime();
		assert_eq!(mtime_before, mtime_after, "mnemonic file was rewritten on second call");
	}

	#[test]
	fn rereads_existing_mnemonic_without_mutation() {
		let dir = tempdir("reread");
		let mnemonic_path = dir.join(DEFAULT_MNEMONIC_FILE);
		fs::write(&mnemonic_path, format!("{}\n", KNOWN_MNEMONIC)).unwrap();
		let bytes_before = fs::read(&mnemonic_path).unwrap();

		load_or_generate_node_entropy(&dir).unwrap();

		let bytes_after = fs::read(&mnemonic_path).unwrap();
		assert_eq!(bytes_before, bytes_after, "mnemonic file content changed");
	}

	#[test]
	fn legacy_keys_seed_is_used_without_generating_a_mnemonic() {
		let dir = tempdir("legacy-seed");
		let seed_path = dir.join(LEGACY_SEED_FILE);
		fs::write(&seed_path, [0x42u8; LEGACY_SEED_LEN]).unwrap();

		assert!(load_legacy_seed(&dir).unwrap().is_some());
		load_or_generate_node_entropy(&dir).unwrap();

		assert!(!dir.join(DEFAULT_MNEMONIC_FILE).exists(), "keys_mnemonic was created");
		assert_eq!(fs::read(&seed_path).unwrap(), [0x42u8; LEGACY_SEED_LEN]);
	}

	#[test]
	fn mnemonic_takes_precedence_over_legacy_keys_seed() {
		let dir = tempdir("both");
		// An unreadable legacy seed would fail the load if it were consulted.
		fs::write(dir.join(LEGACY_SEED_FILE), [0x42u8; 3]).unwrap();
		fs::write(dir.join(DEFAULT_MNEMONIC_FILE), format!("{}\n", KNOWN_MNEMONIC)).unwrap();

		load_or_generate_node_entropy(&dir).unwrap();
	}

	#[test]
	fn rejects_legacy_keys_seed_of_wrong_length() {
		for len in [32, LEGACY_SEED_LEN + 1] {
			let dir = tempdir(&format!("legacy-len-{len}"));
			fs::write(dir.join(LEGACY_SEED_FILE), vec![0x42u8; len]).unwrap();

			let err = load_or_generate_node_entropy(&dir).unwrap_err();
			assert_eq!(err.kind(), io::ErrorKind::InvalidData);
			assert!(!dir.join(DEFAULT_MNEMONIC_FILE).exists(), "keys_mnemonic was created");
		}
	}

	#[test]
	fn rejects_invalid_mnemonic_file() {
		let dir = tempdir("invalid");
		fs::write(
			dir.join(DEFAULT_MNEMONIC_FILE),
			"these words are definitely not a valid bip39 phrase at all nope",
		)
		.unwrap();

		let err = load_or_generate_node_entropy(&dir).unwrap_err();
		assert_eq!(err.kind(), io::ErrorKind::InvalidData);
	}

	#[test]
	fn rejects_oversized_mnemonic_file() {
		let dir = tempdir("oversized");
		fs::write(dir.join(DEFAULT_MNEMONIC_FILE), vec![b'a'; MNEMONIC_FILE_SIZE_LIMIT + 1])
			.unwrap();

		let err = load_or_generate_node_entropy(&dir).unwrap_err();
		assert_eq!(err.kind(), io::ErrorKind::InvalidData);
	}
}
