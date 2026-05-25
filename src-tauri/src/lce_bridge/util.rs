use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read};

pub struct LceReader<'a> {
    cursor: Cursor<&'a [u8]>,
}

impl<'a> LceReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { cursor: Cursor::new(data) }
    }

    pub fn remaining(&self) -> usize {
        self.cursor.get_ref().len() - self.cursor.position() as usize
    }

    pub fn read_u8(&mut self) -> std::io::Result<u8> {
        self.cursor.read_u8()
    }

    pub fn read_i8(&mut self) -> std::io::Result<i8> {
        self.cursor.read_i8()
    }

    pub fn read_u16(&mut self) -> std::io::Result<u16> {
        self.cursor.read_u16::<BigEndian>()
    }

    pub fn read_i16(&mut self) -> std::io::Result<i16> {
        self.cursor.read_i16::<BigEndian>()
    }

    pub fn read_u32(&mut self) -> std::io::Result<u32> {
        self.cursor.read_u32::<BigEndian>()
    }

    pub fn read_i32(&mut self) -> std::io::Result<i32> {
        self.cursor.read_i32::<BigEndian>()
    }

    pub fn read_u64(&mut self) -> std::io::Result<u64> {
        self.cursor.read_u64::<BigEndian>()
    }

    pub fn read_i64(&mut self) -> std::io::Result<i64> {
        self.cursor.read_i64::<BigEndian>()
    }

    pub fn read_f32(&mut self) -> std::io::Result<f32> {
        self.cursor.read_f32::<BigEndian>()
    }

    pub fn read_f64(&mut self) -> std::io::Result<f64> {
        self.cursor.read_f64::<BigEndian>()
    }

    pub fn read_bytes(&mut self, len: usize) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        self.cursor.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_bool(&mut self) -> std::io::Result<bool> {
        Ok(self.cursor.read_u8()? != 0)
    }

    pub fn read_utf16(&mut self, max_length: u32) -> std::io::Result<String> {
        let len = self.read_u16()? as usize;
        if len > max_length as usize {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,
                format!("UTF-16 string too long: {} > {}", len, max_length)));
        }
        let mut chars = Vec::with_capacity(len);
        for _ in 0..len {
            chars.push(self.read_u16()?);
        }
        String::from_utf16(&chars)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn read_string_utf16(&mut self) -> std::io::Result<String> {
        self.read_utf16(0xFFFF)
    }

    pub fn skip(&mut self, count: usize) -> std::io::Result<()> {
        let pos = self.cursor.position() as usize + count;
        if pos > self.cursor.get_ref().len() {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "skip past end"));
        }
        self.cursor.set_position(pos as u64);
        Ok(())
    }
}

pub struct LceWriter {
    buf: Vec<u8>,
}

impl LceWriter {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn write_u8(&mut self, v: u8) {
        let _ = self.buf.write_u8(v);
    }

    pub fn write_i8(&mut self, v: i8) {
        let _ = self.buf.write_i8(v);
    }

    pub fn write_u16(&mut self, v: u16) {
        let _ = self.buf.write_u16::<BigEndian>(v);
    }

    pub fn write_i16(&mut self, v: i16) {
        let _ = self.buf.write_i16::<BigEndian>(v);
    }

    pub fn write_u32(&mut self, v: u32) {
        let _ = self.buf.write_u32::<BigEndian>(v);
    }

    pub fn write_i32(&mut self, v: i32) {
        let _ = self.buf.write_i32::<BigEndian>(v);
    }

    pub fn write_u64(&mut self, v: u64) {
        let _ = self.buf.write_u64::<BigEndian>(v);
    }

    pub fn write_i64(&mut self, v: i64) {
        let _ = self.buf.write_i64::<BigEndian>(v);
    }

    pub fn write_f32(&mut self, v: f32) {
        let _ = self.buf.write_f32::<BigEndian>(v);
    }

    pub fn write_f64(&mut self, v: f64) {
        let _ = self.buf.write_f64::<BigEndian>(v);
    }

    pub fn write_bytes(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    pub fn write_bool(&mut self, v: bool) {
        self.write_u8(if v { 1 } else { 0 });
    }

    pub fn write_utf16(&mut self, s: &str) {
        let encoded: Vec<u16> = s.encode_utf16().collect();
        self.write_u16(encoded.len() as u16);
        for c in encoded {
            self.write_u16(c);
        }
    }
}

pub struct VarInt;

impl VarInt {
    pub fn read(reader: &mut &[u8]) -> std::io::Result<i32> {
        let mut result = 0i32;
        let mut shift = 0;
        loop {
            if reader.is_empty() {
                return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "varint truncated"));
            }
            let byte = reader[0];
            *reader = &reader[1..];
            result |= ((byte & 0x7F) as i32) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
        }
    }

    pub fn write_to(val: i32, buf: &mut Vec<u8>) {
        let mut v = val as u32;
        loop {
            if v & !0x7F == 0 {
                buf.push(v as u8);
                return;
            }
            buf.push((v & 0x7F) as u8 | 0x80);
            v >>= 7;
        }
    }

    pub fn write(val: i32) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut v = val as u32;
        loop {
            if v & !0x7F == 0 {
                buf.push(v as u8);
                return buf;
            }
            buf.push((v & 0x7F) as u8 | 0x80);
            v >>= 7;
        }
    }

    pub fn encode_len(val: i32) -> usize {
        let mut len = 1;
        let mut v = val as u32 >> 7;
        while v != 0 {
            v >>= 7;
            len += 1;
        }
        len
    }
}

pub fn read_varint_from_slice(data: &[u8]) -> std::io::Result<(i32, &[u8])> {
    let mut reader = data;
    let val = VarInt::read(&mut reader)?;
    let consumed = data.len() - reader.len();
    Ok((val, &data[consumed..]))
}
