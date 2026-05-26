use super::Decoder;
use encoding_rs::{BIG5, UTF_8};

#[derive(Debug)]
pub struct Utf8Decoder;

impl Decoder for Utf8Decoder {
    fn decode(&self, bytes: &[u8]) -> anyhow::Result<String> {
        let bytes = strip_bom(bytes);
        let (cow, _, had_errors) = UTF_8.decode(bytes);
        if had_errors {
            anyhow::bail!("utf-8 decode had replacement characters");
        }
        Ok(cow.into_owned())
    }
}

#[derive(Debug)]
pub struct Big5Decoder;

impl Decoder for Big5Decoder {
    fn decode(&self, bytes: &[u8]) -> anyhow::Result<String> {
        let bytes = strip_bom(bytes);
        let (cow, _, had_errors) = BIG5.decode(bytes);
        if had_errors {
            anyhow::bail!("big5 decode had replacement characters");
        }
        Ok(cow.into_owned())
    }
}

#[derive(Debug)]
pub struct BomDecoder;

impl Decoder for BomDecoder {
    fn decode(&self, bytes: &[u8]) -> anyhow::Result<String> {
        if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            Utf8Decoder.decode(bytes)
        } else if bytes.starts_with(&[0xFF, 0xFE]) {
            let (cow, _, errs) = encoding_rs::UTF_16LE.decode(&bytes[2..]);
            if errs {
                anyhow::bail!("utf-16le decode had replacement characters");
            }
            Ok(cow.into_owned())
        } else if bytes.starts_with(&[0xFE, 0xFF]) {
            let (cow, _, errs) = encoding_rs::UTF_16BE.decode(&bytes[2..]);
            if errs {
                anyhow::bail!("utf-16be decode had replacement characters");
            }
            Ok(cow.into_owned())
        } else {
            Utf8Decoder.decode(bytes)
        }
    }
}

fn strip_bom(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_decodes_ascii() {
        let out = Utf8Decoder.decode(b"hello").unwrap();
        assert_eq!(out, "hello");
    }

    #[test]
    fn utf8_strips_bom() {
        let out = Utf8Decoder.decode(b"\xEF\xBB\xBFhi").unwrap();
        assert_eq!(out, "hi");
    }

    #[test]
    fn big5_decodes_cjk() {
        // big5 bytes for 案件
        let bytes = b"\xae\xd7\xa5\xf3";
        let out = Big5Decoder.decode(bytes).unwrap();
        assert_eq!(out, "案件");
    }

    #[test]
    fn bom_chooses_utf8_by_default() {
        let out = BomDecoder.decode(b"hi").unwrap();
        assert_eq!(out, "hi");
    }
}
