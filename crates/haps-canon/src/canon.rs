//! RFC 8785 (JCS) serialization for values in the restricted HAPS domain.
//!
//! Experimental partial implementation; general correctness and totality are
//! open proof obligations. `canonicalize` assumes the parser's value invariants
//! and allocates memory. The integer harness covers only a bounded helper.
//!
//! Byte-oriented output supports Charon/Aeneas extraction experiments.

// Indexing is allowed in this crate for extraction compatibility. The integer
// core has a bounded Kani harness; other indexing paths are not covered by it.
#![allow(clippy::indexing_slicing)]

use alloc::vec::Vec;

use crate::value::Value;

/// Buffer size for [`int_digits`]: 19 digits plus a sign, with room to spare.
pub const INT_BUF: usize = 24;

/// Serialize a value satisfying the parser's restricted-domain invariants.
///
/// Directly constructed values must have valid UTF-8 strings/keys, unique keys
/// already in canonical order, admissible integers and suitable nesting. This
/// function does not validate these conditions or sort objects. Allocation or
/// stack exhaustion is not represented by a returned error.
pub fn canonicalize(value: &Value) -> Vec<u8> {
    write_value(value)
}

/// Each writer *returns* its bytes rather than pushing into a `&mut Vec<u8>`
/// parameter. Aeneas cannot translate a recursive call that aliases a mutable
/// out-parameter — it reports an internal error — but an owned return value
/// translates cleanly. The cost is a copy per nesting level; the gain is that
/// the function has a Lean definition at all.
fn write_value(value: &Value) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    match value {
        Value::Null => {
            out.push(b'n');
            out.push(b'u');
            out.push(b'l');
            out.push(b'l');
        }
        Value::Bool(true) => {
            out.push(b't');
            out.push(b'r');
            out.push(b'u');
            out.push(b'e');
        }
        Value::Bool(false) => {
            out.push(b'f');
            out.push(b'a');
            out.push(b'l');
            out.push(b's');
            out.push(b'e');
        }
        Value::Int(n) => {
            let digits = write_int(*n);
            append(&mut out, &digits);
        }
        Value::Str(s) => {
            let text = write_string(s);
            append(&mut out, &text);
        }
        Value::Array(items) => {
            out.push(b'[');
            let mut i = 0;
            while i < items.len() {
                if i > 0 {
                    out.push(b',');
                }
                let part = write_value(&items[i]);
                append(&mut out, &part);
                i += 1;
            }
            out.push(b']');
        }
        Value::Object(members) => {
            // Parser output is sorted; directly constructed objects are not sorted here.
            out.push(b'{');
            let mut i = 0;
            while i < members.len() {
                if i > 0 {
                    out.push(b',');
                }
                let key = write_string(&members[i].0);
                append(&mut out, &key);
                out.push(b':');
                let part = write_value(&members[i].1);
                append(&mut out, &part);
                i += 1;
            }
            out.push(b'}');
        }
    }
    out
}

// `slice[i]` rather than `slice.get(i)` inside these loops: Aeneas aborts
// translation on `<[T]>::get` in a loop (AeneasVerif/aeneas#1319), and this
// code is intended for extraction. The loop condition guards the index below;
// there is no Kani harness for this function or general panic-freedom proof.
fn append(out: &mut Vec<u8>, bytes: &[u8]) {
    let mut i = 0;
    while i < bytes.len() {
        out.push(bytes[i]);
        i += 1;
    }
}

/// Digits of an admissible integer, most significant first, into a
/// caller-provided buffer. Returns the number of bytes written.
///
/// Intended for integers in ±(2^53 − 1), as accepted by the parser. Direct
/// callers receive no range error. In particular, saturating negation means
/// `i64::MIN` is not serialized faithfully. The Kani harness covers ±10^6 only.
pub fn int_digits(n: i64, buf: &mut [u8; INT_BUF]) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }
    let negative = n < 0;
    let mut magnitude = if negative { n.saturating_neg() } else { n };
    let mut tmp = [0u8; INT_BUF];
    let mut len = 0usize;
    while magnitude > 0 && len < INT_BUF {
        tmp[len] = b'0' + (magnitude % 10) as u8;
        magnitude /= 10;
        len += 1;
    }
    let mut written = 0usize;
    if negative && written < INT_BUF {
        buf[written] = b'-';
        written += 1;
    }
    let mut i = len;
    while i > 0 && written < INT_BUF {
        i -= 1;
        buf[written] = tmp[i];
        written += 1;
    }
    written
}

/// Integers only, so plain decimal: no exponent form, no negative zero.
fn write_int(n: i64) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut buf = [0u8; INT_BUF];
    let len = int_digits(n, &mut buf);
    let mut i = 0;
    while i < len {
        out.push(buf[i]);
        i += 1;
    }
    out
}

/// RFC 8785 string serialization: escape only what must be escaped, pass
/// everything else through as literal UTF-8.
///
/// Every byte needing an escape is ASCII, and no ASCII byte appears inside a
/// multi-byte UTF-8 sequence, so a byte scan is exact: no character decoding
/// is required, and none is done.
fn write_string(s: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.push(b'"');
    let mut i = 0;
    while i < s.len() {
        let b = s[i];
        match b {
            QUOTE => {
                out.push(BACKSLASH);
                out.push(QUOTE);
            }
            BACKSLASH => {
                out.push(BACKSLASH);
                out.push(BACKSLASH);
            }
            0x08 => {
                out.push(BACKSLASH);
                out.push(b'b');
            }
            0x09 => {
                out.push(BACKSLASH);
                out.push(b't');
            }
            0x0A => {
                out.push(BACKSLASH);
                out.push(b'n');
            }
            0x0C => {
                out.push(BACKSLASH);
                out.push(b'f');
            }
            0x0D => {
                out.push(BACKSLASH);
                out.push(b'r');
            }
            _ => {
                if b < 0x20 {
                    out.push(BACKSLASH);
                    out.push(b'u');
                    out.push(b'0');
                    out.push(b'0');
                    out.push(hex_lower(b >> 4));
                    out.push(hex_lower(b & 0x0F));
                } else {
                    out.push(b);
                }
            }
        }
        i += 1;
    }
    out.push(b'"');
    out
}

const QUOTE: u8 = 0x22;
const BACKSLASH: u8 = 0x5C;

const fn hex_lower(nibble: u8) -> u8 {
    if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + (nibble - 10)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::value::MAX_SAFE_INT;

    fn digits(n: i64) -> Vec<u8> {
        let mut buf = [0u8; INT_BUF];
        let len = int_digits(n, &mut buf);
        buf[..len].to_vec()
    }

    /// The admissible-range boundaries, which the bounded Kani harness cannot
    /// reach. These are the values most likely to break a digit loop.
    #[test]
    fn admissible_range_boundaries_serialize_exactly() {
        assert_eq!(digits(MAX_SAFE_INT), b"9007199254740991".to_vec());
        assert_eq!(digits(-MAX_SAFE_INT), b"-9007199254740991".to_vec());
        assert_eq!(digits(0), b"0".to_vec());
        assert_eq!(digits(-1), b"-1".to_vec());
        assert_eq!(digits(10), b"10".to_vec());
        assert_eq!(digits(1_000_000), b"1000000".to_vec());
    }

    /// No exponent form, no plus sign, no negative zero: the three ways a JSON
    /// encoder diverges from JCS on numbers.
    #[test]
    fn integers_canonicalize_without_exponent_or_negative_zero() {
        assert_eq!(canonicalize(&Value::Int(0)), b"0".to_vec());
        assert_eq!(canonicalize(&Value::Int(1_000_000)), b"1000000".to_vec());
        assert_eq!(
            canonicalize(&Value::Int(MAX_SAFE_INT)),
            b"9007199254740991".to_vec()
        );
    }

    #[test]
    fn string_escaping_matches_rfc8785() {
        let s = |b: &[u8]| canonicalize(&Value::Str(b.to_vec()));
        // short escapes for backspace, tab, newline, form feed, carriage return
        assert_eq!(
            s(&[0x08, 0x09, 0x0A, 0x0C, 0x0D]),
            b"\"\\b\\t\\n\\f\\r\"".to_vec()
        );
        // other control characters use lowercase \u00xx
        assert_eq!(s(&[0x01]), b"\"\\u0001\"".to_vec());
        // non-ASCII passes through literally rather than being escaped
        assert_eq!(s("\u{a3}".as_bytes()), "\"\u{a3}\"".as_bytes().to_vec());
        // quote and backslash
        assert_eq!(s(&[QUOTE, BACKSLASH]), b"\"\\\"\\\\\"".to_vec());
    }
}
