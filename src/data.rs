use std::io::Read;

use actix_web::{error::ErrorInternalServerError, web::Buf, Result};
use bytes::{BufMut, Bytes, BytesMut};

pub struct DataReader<'a> {
    pub buf: &'a [u8],
}

impl<'a> DataReader<'a> {
    pub fn new(reader: &'a [u8]) -> Self {
        DataReader { buf: reader }
    }

    pub fn read_int(&mut self) -> i32 {
        self.buf.get_i32()
    }

    pub fn read_int_as_usize(&mut self) -> Result<usize> {
        self.read_int().try_into().map_err(ErrorInternalServerError)
    }

    pub fn read_long(&mut self) -> i64 {
        self.buf.get_i64()
    }

    pub fn read_bool(&mut self) -> bool {
        self.buf.get_u8() != 0
    }

    pub fn read_fully(&mut self, buffer: &mut [u8]) -> Result<()> {
        self.buf.read_exact(buffer)?;
        Ok(())
    }

    pub fn read_utf(&mut self) -> Result<String> {
        let len = self.buf.get_u16().into();
        self.read_utf_of_len(len)
    }

    pub fn read_utf_long(&mut self) -> Result<String> {
        let len = self.read_int_as_usize()?;
        self.read_utf_of_len(len)
    }

    pub fn read_utf_of_len(&mut self, len: usize) -> Result<String> {
        let mut str_bytes = vec![0u8; len];
        self.buf.read_exact(&mut str_bytes)?;
        String::from_utf8(str_bytes).map_err(ErrorInternalServerError)
    }
}

pub struct DataWriter {
    buf: BytesMut,
}

impl DataWriter {
    pub fn new(len: usize) -> Self {
        Self {
            buf: BytesMut::with_capacity(len),
        }
    }

    pub fn write_int(&mut self, value: i32) {
        self.buf.put_i32(value);
    }

    pub fn write_int_from_usize(&mut self, value: usize) -> Result<()> {
        self.buf
            .put_i32(value.try_into().map_err(ErrorInternalServerError)?);
        Ok(())
    }

    pub fn write_long(&mut self, value: i64) {
        self.buf.put_i64(value);
    }

    pub fn write_bool(&mut self, value: bool) {
        self.buf.put_u8(if value { 1 } else { 0 });
    }

    pub fn write_slice(&mut self, value: &[u8]) {
        self.buf.put_slice(value)
    }

    pub fn write_utf_long(&mut self, value: &str) -> Result<()> {
        self.buf
            .put_i32(value.len().try_into().map_err(ErrorInternalServerError)?);
        self.buf.put_slice(value.as_bytes());
        Ok(())
    }

    pub fn write_utf(&mut self, value: &str) -> Result<()> {
        self.buf
            .put_u16(value.len().try_into().map_err(ErrorInternalServerError)?);
        self.buf.put_slice(value.as_bytes());
        Ok(())
    }

    pub fn get_data(self) -> Bytes {
        self.buf.freeze()
    }
}
