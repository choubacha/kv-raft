use bytes::{Buf, BufMut, Bytes, BytesMut, IntoBuf};
use protobuf::{parse_from_bytes, Message, ProtobufError};
use std::marker::PhantomData;
use tokio_codec::{Decoder, Encoder};

/// A frame is a tuple of an integer frame for the first sequence
/// followed by a run of bytes that is the length of the first sequence.
#[derive(Debug)]
pub struct Proto<T: Message> {
    len: Option<u32>,
    phantom: PhantomData<T>,
}

impl<T: Message> Proto<T> {
    pub fn new() -> Self {
        Proto {
            len: None,
            phantom: PhantomData,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Io(::std::io::Error),
    InvalidByteSequence,
    Proto(ProtobufError),
}

impl From<::std::io::Error> for Error {
    fn from(io: ::std::io::Error) -> Error {
        Error::Io(io)
    }
}

impl From<ProtobufError> for Error {
    fn from(io: ProtobufError) -> Error {
        Error::Proto(io)
    }
}

impl<T: Message> Decoder for Proto<T> {
    type Item = T;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        const HEADER: usize = 4;

        // Input must be at least 4 bytes to include content length
        if src.len() >= HEADER {
            // Peek at the length without changing src.
            let content_len = Bytes::from(&src[0..HEADER]).into_buf().get_u32_be() as usize;
            let frame_len = HEADER + content_len;

            if src.len() >= frame_len {
                let bytes = src.split_to(frame_len);

                return parse_from_bytes::<Self::Item>(&bytes[HEADER..])
                    .map(|p| Some(p))
                    .map_err(Error::from);
            }
        }

        Ok(None)
    }
}

impl<T: Message> Encoder for Proto<T> {
    type Item = T;
    type Error = Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let data = item.write_to_bytes()?;
        dst.put_u32_be(data.len() as u32);
        dst.put_slice(&data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use public::{request, Request};

    #[test]
    fn test_decoding() {
        let mut req = Request::new();
        let mut get = request::Get::new();
        get.set_key(String::from("hello"));
        req.set_get(get);

        let proto = req.write_to_bytes().unwrap();

        let mut complete = BytesMut::new();

        complete.put_u32_be(proto.len() as u32);
        complete.put_slice(&proto);

        let mut incomplete = complete.clone().split_to(6);

        let mut decoder = Proto::<Request>::new();
        assert!(decoder.decode(&mut incomplete).unwrap().is_none());

        assert_eq!(decoder.decode(&mut complete).unwrap().unwrap(), req);

        // Test a zero length buffer
        let mut zero = BytesMut::new();
        zero.put_u32_be(0);

        assert_eq!(decoder.decode(&mut zero).unwrap().unwrap(), Request::new());
    }

    #[test]
    fn test_encoding() {
        let mut req = Request::new();
        let mut get = request::Get::new();
        get.set_key(String::from("hello"));
        req.set_get(get);

        let proto = req.write_to_bytes().unwrap();

        let mut expected = BytesMut::new();

        expected.put_u32_be(proto.len() as u32);
        expected.put_slice(&proto);

        let mut actual = BytesMut::new();
        Proto::<Request>::new().encode(req, &mut actual).unwrap();
        assert_eq!(actual, expected);
    }
}
