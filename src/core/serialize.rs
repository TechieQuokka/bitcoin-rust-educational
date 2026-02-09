// Serialization utilities for Bitcoin data structures

use std::io::{self, Read, Write};

/// Trait for serializable types
pub trait Serializable {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(data: &[u8]) -> Result<Self, String> where Self: Sized;
}

/// Write a variable-length integer (VarInt)
/// Bitcoin uses a compact format for integers
pub fn write_varint<W: Write>(writer: &mut W, value: u64) -> io::Result<()> {
    match value {
        0..=0xfc => {
            writer.write_all(&[value as u8])?;
        }
        0xfd..=0xffff => {
            writer.write_all(&[0xfd])?;
            writer.write_all(&(value as u16).to_le_bytes())?;
        }
        0x10000..=0xffffffff => {
            writer.write_all(&[0xfe])?;
            writer.write_all(&(value as u32).to_le_bytes())?;
        }
        _ => {
            writer.write_all(&[0xff])?;
            writer.write_all(&value.to_le_bytes())?;
        }
    }
    Ok(())
}

/// Read a variable-length integer (VarInt)
pub fn read_varint<R: Read + ?Sized>(reader: &mut R) -> io::Result<u64> {
    let mut first_byte = [0u8; 1];
    reader.read_exact(&mut first_byte)?;

    match first_byte[0] {
        0..=0xfc => Ok(first_byte[0] as u64),
        0xfd => {
            let mut bytes = [0u8; 2];
            reader.read_exact(&mut bytes)?;
            Ok(u16::from_le_bytes(bytes) as u64)
        }
        0xfe => {
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            Ok(u32::from_le_bytes(bytes) as u64)
        }
        0xff => {
            let mut bytes = [0u8; 8];
            reader.read_exact(&mut bytes)?;
            Ok(u64::from_le_bytes(bytes))
        }
    }
}

/// Write bytes with length prefix (VarInt length + data)
pub fn write_var_bytes<W: Write>(writer: &mut W, data: &[u8]) -> io::Result<()> {
    write_varint(writer, data.len() as u64)?;
    writer.write_all(data)?;
    Ok(())
}

/// Read bytes with length prefix
pub fn read_var_bytes<R: Read + ?Sized>(reader: &mut R) -> io::Result<Vec<u8>> {
    let len = read_varint(reader)? as usize;
    let mut data = vec![0u8; len];
    reader.read_exact(&mut data)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_varint_small() {
        let mut buf = Vec::new();
        write_varint(&mut buf, 100).unwrap();
        assert_eq!(buf, vec![100]);

        let mut cursor = Cursor::new(buf);
        let value = read_varint(&mut cursor).unwrap();
        assert_eq!(value, 100);
    }

    #[test]
    fn test_varint_medium() {
        let mut buf = Vec::new();
        write_varint(&mut buf, 1000).unwrap();
        assert_eq!(buf.len(), 3); // 0xfd + 2 bytes

        let mut cursor = Cursor::new(buf);
        let value = read_varint(&mut cursor).unwrap();
        assert_eq!(value, 1000);
    }

    #[test]
    fn test_varint_large() {
        let mut buf = Vec::new();
        write_varint(&mut buf, 100000).unwrap();
        assert_eq!(buf.len(), 5); // 0xfe + 4 bytes

        let mut cursor = Cursor::new(buf);
        let value = read_varint(&mut cursor).unwrap();
        assert_eq!(value, 100000);
    }

    #[test]
    fn test_var_bytes() {
        let data = b"hello world";
        let mut buf = Vec::new();
        write_var_bytes(&mut buf, data).unwrap();

        let mut cursor = Cursor::new(buf);
        let decoded = read_var_bytes(&mut cursor).unwrap();
        assert_eq!(decoded, data);
    }
}
