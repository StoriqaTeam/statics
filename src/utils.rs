use std::io::{Error as StdError, Read};
use std::cmp;

pub struct Bytes {
    pub inner: Vec<u8>,
}

impl Bytes {
    pub fn new(data: Vec<u8>) -> Self {
        Self { inner: data }
    }
}

impl Read for Bytes {
    // Todo - this method needs optimization, as its probably doing more copies than necessary
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, StdError> {
        let amt = cmp::min(buf.len(), self.inner.len());
        let bytes = self.inner.drain(..amt).collect::<Vec<u8>>();
        buf[..amt].copy_from_slice(&bytes[..]);

        Ok(amt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vecex_read() {
        let mut ex1 = Bytes::new(vec![1, 2, 3, 4, 5]);
        let buf = &mut vec![0, 0];
        assert_eq!(ex1.read(buf).unwrap(), 2);
        assert_eq!(buf, &vec![1, 2]);
        assert_eq!(ex1.inner, vec![3u8, 4u8, 5u8]);

        assert_eq!(ex1.read(buf).unwrap(), 2);
        assert_eq!(buf, &vec![3, 4]);
        assert_eq!(ex1.inner, vec![5u8]);

        assert_eq!(ex1.read(buf).unwrap(), 1);
        assert_eq!(buf, &vec![5, 4]);
        assert_eq!(ex1.inner, vec![] as Vec<u8>);

        assert_eq!(ex1.read(buf).unwrap(), 0);
        assert_eq!(buf, &vec![5, 4]);
        assert_eq!(ex1.inner, vec![] as Vec<u8>);
    }
}
