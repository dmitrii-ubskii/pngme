use std::{
	fs::File,
	io::{Read, Write},
	path::PathBuf,
};

use chunk::Chunk;
use clap::{Parser, Subcommand};
use png::Png;

mod args;
mod chunk;
mod chunk_type;
mod commands;
mod png;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	Encode { filename: PathBuf, chunk_type: String, message: String },
	Decode { filename: PathBuf, chunk_type: String },
	Remove { filename: PathBuf, chunk_type: String },
	Print { filename: PathBuf },
}

fn main() -> Result<()> {
	let cli = Cli::parse();

	match cli.command {
		Commands::Encode { filename, chunk_type, message } => {
			let mut buf = Vec::new();
			File::open(&filename)?.read_to_end(&mut buf)?;
			let mut png = Png::try_from(buf.as_slice())?;
			png.append_chunk(Chunk::new(chunk_type.parse()?, message.as_bytes().to_vec()));
			File::create(filename)?.write_all(&png.as_bytes())?;
		}
		Commands::Decode { filename, chunk_type } => {
			let mut buf = Vec::new();
			File::open(filename)?.read_to_end(&mut buf)?;
			if let Some(chunk) = Png::try_from(buf.as_slice())?.chunk_by_type(&chunk_type) {
				println!("{}", chunk);
			}
		}
		Commands::Remove { filename, chunk_type } => {
			let mut buf = Vec::new();
			File::open(&filename)?.read_to_end(&mut buf)?;
			let mut png = Png::try_from(buf.as_slice())?;
			png.remove_chunk(&chunk_type)?;
			File::create(filename)?.write_all(&png.as_bytes())?;
		}
		Commands::Print { filename } => {
			let mut buf = Vec::new();
			File::open(filename)?.read_to_end(&mut buf)?;
			for chunk in Png::try_from(buf.as_slice())?.chunks() {
				if let Ok(string) = chunk.data_as_string() {
					println!("{}\t{}", chunk.chunk_type(), string);
				}
			}
		}
	}

	Ok(())
}
