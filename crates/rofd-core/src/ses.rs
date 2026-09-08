//! Minimal DER reader for SES_Signature (`SignedValue.dat`) seal pictures.
//!
//! Only the pieces needed to locate the electronic-seal picture are
//! implemented: single-byte tags, short- and long-form lengths, and recursive
//! constructed children. Cryptographic verification is out of scope.

use crate::{Error, Result};

const TAG_INTEGER: u8 = 0x02;
const TAG_OCTET_STRING: u8 = 0x04;
const TAG_IA5_STRING: u8 = 0x16;
const TAG_SEQUENCE: u8 = 0x30;
const CONSTRUCTED_BIT: u8 = 0x20;
const MAX_DEPTH: usize = 16;
const MAX_NODES: usize = 4096;

/// The seal picture extracted from one SES_Signature value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SealPictureData {
    /// Picture type string declared by the seal (for example `ofd` or `png`).
    pub(crate) kind: String,
    /// Encoded picture bytes.
    pub(crate) data: Vec<u8>,
    /// Declared picture width in millimetres, when small enough to represent.
    pub(crate) width: Option<u32>,
    /// Declared picture height in millimetres, when small enough to represent.
    pub(crate) height: Option<u32>,
}

#[derive(Debug)]
struct DerNode<'a> {
    tag: u8,
    content: &'a [u8],
    children: Vec<DerNode<'a>>,
}

struct DerParser<'a> {
    bytes: &'a [u8],
    nodes: usize,
}

impl<'a> DerParser<'a> {
    fn parse_node(&mut self, offset: usize, depth: usize) -> Result<(DerNode<'a>, usize)> {
        if depth > MAX_DEPTH {
            return Err(invalid("nesting depth exceeds the supported maximum"));
        }
        self.nodes = self.nodes.saturating_add(1);
        if self.nodes > MAX_NODES {
            return Err(invalid("node count exceeds the supported maximum"));
        }
        let tag = *self
            .bytes
            .get(offset)
            .ok_or_else(|| invalid("truncated tag"))?;
        if tag & 0x1f == 0x1f {
            return Err(invalid("high-tag-number form is not supported"));
        }
        let constructed = tag & CONSTRUCTED_BIT != 0;
        let length_byte = *self
            .bytes
            .get(offset.saturating_add(1))
            .ok_or_else(|| invalid("truncated length"))?;
        let mut cursor = offset.saturating_add(2);
        let length = if length_byte & 0x80 == 0 {
            usize::from(length_byte)
        } else {
            let octets = usize::from(length_byte & 0x7f);
            if octets == 0 {
                return Err(invalid("indefinite length is not valid DER"));
            }
            if octets > 4 {
                return Err(invalid("length uses more than four octets"));
            }
            let mut length = 0usize;
            for index in 0..octets {
                let byte = *self
                    .bytes
                    .get(cursor.saturating_add(index))
                    .ok_or_else(|| invalid("truncated long-form length"))?;
                length = length
                    .checked_shl(8)
                    .and_then(|value| value.checked_add(usize::from(byte)))
                    .ok_or_else(|| invalid("length overflow"))?;
            }
            cursor = cursor.saturating_add(octets);
            length
        };
        let end = cursor
            .checked_add(length)
            .ok_or_else(|| invalid("content length overflow"))?;
        let content = self
            .bytes
            .get(cursor..end)
            .ok_or_else(|| invalid("truncated content"))?;
        let mut children = Vec::new();
        if constructed {
            let mut child_offset = cursor;
            while child_offset < end {
                let (child, next) = self.parse_node(child_offset, depth.saturating_add(1))?;
                children.push(child);
                child_offset = next;
            }
        }
        Ok((
            DerNode {
                tag,
                content,
                children,
            },
            end,
        ))
    }
}

fn invalid(message: &str) -> Error {
    Error::InvalidSignatureValue(message.to_owned())
}

/// Extracts the seal picture from DER-encoded SES_Signature bytes.
///
/// The layout navigated is `SES_Signature` → `toSign` (first child), beneath
/// which the `picture` sequence is located structurally rather than by fixed
/// index: producers wrap the seal in different intermediate sequences, so the
/// search descends recursively until a sequence shaped as `[IA5String type,
/// OCTET STRING data, INTEGER width, INTEGER height]` is found.
pub(crate) fn parse_seal_picture(bytes: &[u8]) -> Result<SealPictureData> {
    let mut parser = DerParser { bytes, nodes: 0 };
    let (root, _) = parser.parse_node(0, 0)?;
    if root.tag != TAG_SEQUENCE {
        return Err(invalid("root is not a SEQUENCE"));
    }
    let to_sign = root
        .children
        .first()
        .filter(|node| node.tag == TAG_SEQUENCE)
        .ok_or_else(|| invalid("toSign sequence is missing"))?;
    for child in &to_sign.children {
        if let Some(picture) = find_picture(child) {
            return picture;
        }
    }
    Err(invalid("no seal picture found"))
}

fn find_picture(node: &DerNode<'_>) -> Option<Result<SealPictureData>> {
    if let Some(picture) = picture_from(node) {
        return Some(picture);
    }
    if node.tag == TAG_SEQUENCE {
        for child in &node.children {
            if let Some(picture) = find_picture(child) {
                return Some(picture);
            }
        }
    }
    None
}

fn picture_from(node: &DerNode<'_>) -> Option<Result<SealPictureData>> {
    if node.tag != TAG_SEQUENCE || node.children.len() < 4 {
        return None;
    }
    let [kind, data, width, height] = &node.children[..4] else {
        return None;
    };
    if kind.tag != TAG_IA5_STRING
        || data.tag != TAG_OCTET_STRING
        || width.tag != TAG_INTEGER
        || height.tag != TAG_INTEGER
    {
        return None;
    }
    Some((|| {
        let kind = String::from_utf8(kind.content.to_vec())
            .map_err(|_| invalid("picture type is not valid IA5 text"))?;
        Ok(SealPictureData {
            kind,
            data: data.content.to_vec(),
            width: integer_value(width.content),
            height: integer_value(height.content),
        })
    })())
}

fn integer_value(content: &[u8]) -> Option<u32> {
    if content.is_empty() || content.len() > 4 {
        return None;
    }
    content.iter().try_fold(0u32, |value, byte| {
        value.checked_shl(8)?.checked_add(u32::from(*byte))
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_seal_picture, SealPictureData};
    use crate::Error;

    fn der(tag: u8, content: &[u8]) -> Vec<u8> {
        let mut out = vec![tag];
        if content.len() < 0x80 {
            out.push(content.len() as u8);
        } else {
            let length = content.len();
            let octets = length.to_be_bytes();
            let significant = octets
                .iter()
                .position(|byte| *byte != 0)
                .expect("nonzero length");
            let significant = &octets[significant..];
            out.push(0x80 | significant.len() as u8);
            out.extend_from_slice(significant);
        }
        out.extend_from_slice(content);
        out
    }

    fn sequence(children: &[Vec<u8>]) -> Vec<u8> {
        der(0x30, &children.concat())
    }

    fn seal_bytes(kind: &str, data: &[u8], long_form: bool) -> Vec<u8> {
        let picture = sequence(&[
            der(0x16, kind.as_bytes()),
            der(0x04, data),
            der(0x02, &[30]),
            der(0x02, &[20]),
        ]);
        let eseal = sequence(&[
            sequence(&[der(0x16, b"ES"), der(0x02, &[4])]),
            der(0x16, b"seal-id"),
            picture,
        ]);
        let to_sign = if long_form {
            // Force a long-form length on toSign even when unnecessary.
            let content = [der(0x02, &[1]), eseal].concat();
            let mut encoded = vec![0x30, 0x82];
            encoded.extend_from_slice(&(content.len() as u16).to_be_bytes());
            encoded.extend_from_slice(&content);
            encoded
        } else {
            sequence(&[der(0x02, &[1]), eseal])
        };
        sequence(&[to_sign, der(0x06, &[0x2a])])
    }

    #[test]
    fn extracts_picture_type_data_and_dimensions() {
        let picture = parse_seal_picture(&seal_bytes("ofd", b"PK\x03\x04", false)).unwrap();
        assert_eq!(
            picture,
            SealPictureData {
                kind: "ofd".to_owned(),
                data: b"PK\x03\x04".to_vec(),
                width: Some(30),
                height: Some(20),
            }
        );
    }

    #[test]
    fn supports_long_form_lengths() {
        let data = vec![0xabu8; 300];
        let picture = parse_seal_picture(&seal_bytes("png", &data, true)).unwrap();
        assert_eq!(picture.kind, "png");
        assert_eq!(picture.data, data);
        assert_eq!(picture.width, Some(30));
    }

    #[test]
    fn rejects_truncated_and_garbage_input_without_panicking() {
        let valid = seal_bytes("png", &[1, 2, 3], true);
        for truncated_len in 0..valid.len() {
            assert!(parse_seal_picture(&valid[..truncated_len]).is_err());
        }
        assert!(parse_seal_picture(&[]).is_err());
        assert!(parse_seal_picture(&[0x30]).is_err());
        assert!(parse_seal_picture(&[0x30, 0x80, 0x00]).is_err());
        assert!(parse_seal_picture(&[0x04, 0x03, 1, 2, 3]).is_err());
        assert!(parse_seal_picture(&[0x30, 0xff]).is_err());
        assert!(parse_seal_picture(&[0x1f, 0x00, 0x00]).is_err());
    }

    #[test]
    fn rejects_input_without_a_picture() {
        let to_sign = sequence(&[der(0x02, &[1]), sequence(&[der(0x16, b"ES")])]);
        let bytes = sequence(&[to_sign]);
        let error = parse_seal_picture(&bytes).unwrap_err();
        assert!(matches!(error, Error::InvalidSignatureValue(_)));
    }

    #[test]
    fn finds_picture_behind_intermediate_seal_wrappers() {
        // Real producers (for example the 999.ofd fixture) wrap header, esID,
        // property, and picture in an extra sequence inside eseal, followed by
        // the certificate, algorithm, and signature values.
        let picture = sequence(&[
            der(0x16, b"gif"),
            der(0x04, b"GIF89a"),
            der(0x02, &[45]),
            der(0x02, &[45]),
        ]);
        let seal_info = sequence(&[
            sequence(&[sequence(&[der(0x16, b"ES")])]),
            der(0x16, b"seal-id"),
            sequence(&[der(0x02, &[3]), der(0x04, b"cert-inside-property")]),
            picture,
            sequence(&[]),
        ]);
        let eseal = sequence(&[
            seal_info,
            der(0x04, b"cert"),
            der(0x06, &[0x2a]),
            der(0x03, b"\x00sig"),
        ]);
        let to_sign = sequence(&[
            der(0x02, &[1]),
            eseal,
            der(0x18, b"20200817111329Z"),
            der(0x03, b"\x00hash"),
            der(0x16, b"/Doc_0/Signs/Sign_0/Signature.xml"),
        ]);
        let bytes = sequence(&[to_sign, der(0x04, b"cert"), der(0x06, &[0x2a])]);
        let picture = parse_seal_picture(&bytes).unwrap();
        assert_eq!(picture.kind, "gif");
        assert_eq!(picture.data, b"GIF89a");
        assert_eq!(picture.width, Some(45));
        assert_eq!(picture.height, Some(45));
    }
}
