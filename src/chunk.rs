use core::fmt;
use std::{mem, str, sync::OnceLock};

use crate::{
	chunk_type::{self, ChunkType},
	Error, Result,
};

pub struct Chunk {
	chunk_type: ChunkType,
	data: Vec<u8>,
	crc: u32,
}

impl Chunk {
	pub fn new(chunk_type: ChunkType, data: Vec<u8>) -> Self {
		let mut bytes = Vec::with_capacity(chunk_type.bytes().len() + data.len());
		bytes.extend_from_slice(&chunk_type.bytes());
		bytes.extend_from_slice(&data);
		let crc = compute_crc(&bytes);

		Self { chunk_type, data, crc }
	}
	pub fn length(&self) -> u32 {
		self.data.len() as u32
	}
	pub fn chunk_type(&self) -> &ChunkType {
		&self.chunk_type
	}
	pub fn data(&self) -> &[u8] {
		&self.data
	}
	pub fn crc(&self) -> u32 {
		self.crc
	}
	pub fn data_as_string(&self) -> Result<String> {
		Ok(str::from_utf8(self.data())?.to_owned())
	}
	pub fn as_bytes(&self) -> Vec<u8> {
		((self.data.len() as u32).to_be_bytes().iter())
			.chain(self.chunk_type.bytes().iter())
			.chain(self.data.iter())
			.chain(self.crc.to_be_bytes().iter())
			.copied()
			.collect()
	}
}

fn compute_crc(bytes: &[u8]) -> u32 {
	static CRC_TABLE: OnceLock<[u32; 256]> = OnceLock::new();
	let crc_table = CRC_TABLE.get_or_init(|| {
		let mut buf = [0; 256];
		buf.iter_mut().enumerate().for_each(|(n, x)| {
			let mut c = n as u32;
			for _ in 0..8 {
				if (c & 1) != 0 {
					c = 0xedb88320 ^ (c >> 1);
				} else {
					c >>= 1;
				}
			}
			*x = c;
		});
		buf
	});

	let mut crc = 0xffffffff;
	for byte in bytes {
		crc = crc_table[((crc ^ *byte as u32) & 0xff) as usize] ^ (crc >> 8);
	}
	crc ^ 0xffffffff
}

#[derive(Debug)]
struct InvalidChunkLength {
	expected: u32,
	received: u32,
}
impl std::error::Error for InvalidChunkLength {}
impl fmt::Display for InvalidChunkLength {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Invalid chunk length: expected {}, got {}", self.expected, self.received)
	}
}

#[derive(Debug)]
struct InvalidChunkCrc {
	expected: u32,
	computed: u32,
}
impl std::error::Error for InvalidChunkCrc {}
impl fmt::Display for InvalidChunkCrc {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Invalid chunk crc: expected 0x{:x}, got 0x{:x}", self.expected, self.computed)
	}
}

impl TryFrom<&[u8]> for Chunk {
	type Error = Error;
	fn try_from(bytes: &[u8]) -> Result<Self> {
		let (len, bytes) = bytes.split_at(4);
		let len = u32::from_be_bytes(len.try_into()?);

		if len as usize + mem::size_of::<(u32, u32)>() != bytes.len() {
			return Err(InvalidChunkLength {
				expected: len,
				received: (bytes.len() - mem::size_of::<(u32, u32)>()) as u32,
			}
			.into());
		}

		let (bytes, crc) = bytes.split_at(bytes.len() - 4);
		let computed_crc = compute_crc(bytes);
		let crc = u32::from_be_bytes(crc.try_into()?);
		if computed_crc != crc {
			return Err(InvalidChunkCrc { expected: crc, computed: computed_crc }.into());
		}

		let (chunk_type, chunk_data) = bytes.split_at(4);
		let chunk_type = ChunkType::try_from(<[u8; 4]>::try_from(chunk_type)?)?;

		Ok(Self::new(chunk_type, chunk_data.to_owned()))
	}
}

impl fmt::Display for Chunk {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.data_as_string().map_err(|_| fmt::Error)?)
	}
}

#[cfg(test)]
mod tests {
	use std::str::FromStr;

	use super::*;
	use crate::chunk_type::ChunkType;

	fn testing_chunk() -> Chunk {
		let data_length: u32 = 42;
		let chunk_type = "RuSt".as_bytes();
		let message_bytes = "This is where your secret message will be!".as_bytes();
		let crc: u32 = 2882656334;

		let chunk_data: Vec<u8> = data_length
			.to_be_bytes()
			.iter()
			.chain(chunk_type.iter())
			.chain(message_bytes.iter())
			.chain(crc.to_be_bytes().iter())
			.copied()
			.collect();

		Chunk::try_from(chunk_data.as_ref()).unwrap()
	}

	#[test]
	fn test_new_chunk() {
		let chunk_type = ChunkType::from_str("RuSt").unwrap();
		let data = "This is where your secret message will be!".as_bytes().to_vec();
		let chunk = Chunk::new(chunk_type, data);
		assert_eq!(chunk.length(), 42);
		assert_eq!(chunk.crc(), 2882656334);
	}

	#[test]
	fn test_chunk_length() {
		let chunk = testing_chunk();
		assert_eq!(chunk.length(), 42);
	}

	#[test]
	fn test_chunk_type() {
		let chunk = testing_chunk();
		assert_eq!(chunk.chunk_type().to_string(), String::from("RuSt"));
	}

	#[test]
	fn test_chunk_string() {
		let chunk = testing_chunk();
		let chunk_string = chunk.data_as_string().unwrap();
		let expected_chunk_string = String::from("This is where your secret message will be!");
		assert_eq!(chunk_string, expected_chunk_string);
	}

	#[test]
	fn test_chunk_crc() {
		let chunk = testing_chunk();
		assert_eq!(chunk.crc(), 2882656334);
	}

	#[test]
	fn test_valid_chunk_from_bytes() {
		let data_length: u32 = 42;
		let chunk_type = "RuSt".as_bytes();
		let message_bytes = "This is where your secret message will be!".as_bytes();
		let crc: u32 = 2882656334;

		let chunk_data: Vec<u8> = data_length
			.to_be_bytes()
			.iter()
			.chain(chunk_type.iter())
			.chain(message_bytes.iter())
			.chain(crc.to_be_bytes().iter())
			.copied()
			.collect();

		let chunk = Chunk::try_from(chunk_data.as_ref()).unwrap();

		let chunk_string = chunk.data_as_string().unwrap();
		let expected_chunk_string = String::from("This is where your secret message will be!");

		assert_eq!(chunk.length(), 42);
		assert_eq!(chunk.chunk_type().to_string(), String::from("RuSt"));
		assert_eq!(chunk_string, expected_chunk_string);
		assert_eq!(chunk.crc(), 2882656334);
	}

	#[test]
	fn test_invalid_chunk_from_bytes() {
		let data_length: u32 = 42;
		let chunk_type = "RuSt".as_bytes();
		let message_bytes = "This is where your secret message will be!".as_bytes();
		let crc: u32 = 2882656333;

		let chunk_data: Vec<u8> = data_length
			.to_be_bytes()
			.iter()
			.chain(chunk_type.iter())
			.chain(message_bytes.iter())
			.chain(crc.to_be_bytes().iter())
			.copied()
			.collect();

		let chunk = Chunk::try_from(chunk_data.as_ref());

		assert!(chunk.is_err());
	}

	#[test]
	pub fn test_chunk_trait_impls() {
		let data_length: u32 = 42;
		let chunk_type = "RuSt".as_bytes();
		let message_bytes = "This is where your secret message will be!".as_bytes();
		let crc: u32 = 2882656334;

		let chunk_data: Vec<u8> = data_length
			.to_be_bytes()
			.iter()
			.chain(chunk_type.iter())
			.chain(message_bytes.iter())
			.chain(crc.to_be_bytes().iter())
			.copied()
			.collect();

		let chunk: Chunk = TryFrom::try_from(chunk_data.as_ref()).unwrap();

		let _chunk_string = format!("{}", chunk);
	}
}
