pub mod client;
pub mod server;

pub use client::JormungandrClient;
pub use server::JormungandrServerImpl;

use chain_core::mempack::{ReadBuf, Readable};

pub fn read_into<T: Readable>(bytes: &[u8]) -> T {
    let mut buf = ReadBuf::from(bytes);
    T::read(&mut buf).unwrap()
}
