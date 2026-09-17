//! Strict JSON parser for the restricted HAPS value domain.
//!
//! It rejects duplicate member names, fractional/exponent numbers, and integers
//! outside the admissible range before returning a value. Unit tests cover
//! selected P3 rejection cases; general parser correctness is not proved.
//!
//! UTF-8 bytes keep the implementation accessible to extraction experiments.
//! Parsing returns typed syntax/domain errors and limits nesting, but still
//! allocates memory and has no byte or collection-size limit. This is not a
//! general termination, panic-freedom or resource-exhaustion guarantee.

use alloc::vec::Vec;

use crate::order::cmp_canonical;
use crate::value::{Value, MAX_SAFE_INT};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// A number had a fractional part or an exponent (`haps-v0.4.0.md` §6.1).
    NonIntegerNumber,
    /// An integer outside ±(2^53 − 1).
    IntegerOutOfRange,
    /// The same member name appeared twice in one object.
    DuplicateKey,
    /// Structurally invalid JSON.
    Syntax,
    /// Well-formed JSON followed by trailing content.
    TrailingContent,
    /// Nesting deeper than [`MAX_DEPTH`].
    DepthExceeded,
}

/// Maximum nesting depth checked by the parser. This limits recursive parsing
/// but does not bound input size, allocation, total work or caller stack usage.
pub const MAX_DEPTH: u32 = 64;

pub fn parse(input: &str) -> Result<Value, ParseError> {
    let mut p = Parser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    p.skip_ws();
    let value = p.parse_value(0)?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(ParseError::TrailingContent);
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    // Indexing rather than `get`: Aeneas aborts translation on `<[T]>::get`
    // inside a loop (AeneasVerif/aeneas#1319). Each index is guarded by an
    // explicit bounds test; these helpers have no dedicated Kani harness.
    fn peek(&self) -> Option<u8> {
        if self.pos < self.bytes.len() {
            Some(self.bytes[self.pos])
        } else {
            None
        }
    }

    fn bump(&mut self) -> Option<u8> {
        if self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            self.pos += 1;
            Some(b)
        } else {
            None
        }
    }

    fn eat(&mut self, expected: u8) -> Result<(), ParseError> {
        match self.peek() {
            Some(b) if b == expected => {
                self.pos += 1;
                Ok(())
            }
            _ => Err(ParseError::Syntax),
        }
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            match b {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn literal(&mut self, word: &[u8]) -> Result<(), ParseError> {
        let end = self.pos.saturating_add(word.len());
        if end > self.bytes.len() {
            return Err(ParseError::Syntax);
        }
        let mut k = 0usize;
        while k < word.len() {
            if self.bytes[self.pos + k] != word[k] {
                return Err(ParseError::Syntax);
            }
            k += 1;
        }
        self.pos = end;
        Ok(())
    }

    fn parse_value(&mut self, depth: u32) -> Result<Value, ParseError> {
        if depth > MAX_DEPTH {
            return Err(ParseError::DepthExceeded);
        }
        match self.peek() {
            None => Err(ParseError::Syntax),
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(Value::Null)
            }
            Some(b't') => {
                self.literal(b"true")?;
                Ok(Value::Bool(true))
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(Value::Bool(false))
            }
            Some(b'"') => Ok(Value::Str(self.parse_string()?)),
            Some(b'[') => self.parse_array(depth),
            Some(b'{') => self.parse_object(depth),
            Some(b'-') => self.parse_number(),
            Some(b) if b.is_ascii_digit() => self.parse_number(),
            Some(_) => Err(ParseError::Syntax),
        }
    }

    fn parse_array(&mut self, depth: u32) -> Result<Value, ParseError> {
        self.eat(b'[')?;
        let mut items: Vec<Value> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.skip_ws();
            let v = self.parse_value(depth.saturating_add(1))?;
            items.push(v);
            self.skip_ws();
            match self.bump() {
                Some(b',') => continue,
                Some(b']') => return Ok(Value::Array(items)),
                _ => return Err(ParseError::Syntax),
            }
        }
    }

    fn parse_object(&mut self, depth: u32) -> Result<Value, ParseError> {
        self.eat(b'{')?;
        let mut members: Vec<(Vec<u8>, Value)> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(members));
        }
        let mut finished = false;
        while !finished {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.eat(b':')?;
            self.skip_ws();
            let value = self.parse_value(depth.saturating_add(1))?;

            // P3: duplicate member names are rejected, not resolved last-wins.
            let mut i = 0;
            while i < members.len() {
                if cmp_canonical(key.as_slice(), members[i].0.as_slice()).is_eq() {
                    return Err(ParseError::DuplicateKey);
                }
                i += 1;
            }
            // Insert in canonical order, so an Object is sorted by construction
            // and canonicalization never has to sort.
            let mut at = members.len();
            let mut j = 0;
            while j < members.len() {
                if cmp_canonical(key.as_slice(), members[j].0.as_slice()).is_lt() {
                    at = j;
                    break;
                }
                j += 1;
            }
            members.insert(at, (key, value));

            self.skip_ws();
            match self.bump() {
                Some(b',') => {}
                Some(b'}') => finished = true,
                _ => return Err(ParseError::Syntax),
            }
        }
        Ok(Value::Object(members))
    }

    fn parse_number(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        // Integer part: `0` alone, or a non-zero digit followed by digits.
        match self.peek() {
            Some(b'0') => {
                self.pos += 1;
                if let Some(b) = self.peek() {
                    if b.is_ascii_digit() {
                        return Err(ParseError::Syntax); // leading zero
                    }
                }
            }
            Some(b) if b.is_ascii_digit() => {
                while let Some(d) = self.peek() {
                    if d.is_ascii_digit() {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
            }
            _ => return Err(ParseError::Syntax),
        }
        // §6.1: a fractional part or exponent is not admissible. Reject with a
        // specific error rather than a generic one, so an implementer sees why.
        if let Some(b) = self.peek() {
            if b == b'.' || b == b'e' || b == b'E' {
                return Err(ParseError::NonIntegerNumber);
            }
        }
        if self.pos > self.bytes.len() || start > self.pos {
            return Err(ParseError::Syntax);
        }
        let n = parse_int(&self.bytes[start..self.pos])?;
        if !(-MAX_SAFE_INT..=MAX_SAFE_INT).contains(&n) {
            return Err(ParseError::IntegerOutOfRange);
        }
        // RFC 8785 serializes negative zero as `0`; with integers only, -0 and 0
        // are the same value, so normalise and keep canonical form a function.
        Ok(Value::Int(if n == 0 { 0 } else { n }))
    }

    fn parse_string(&mut self) -> Result<Vec<u8>, ParseError> {
        self.eat(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            match self.bump() {
                None => return Err(ParseError::Syntax),
                Some(b'"') => return Ok(out),
                Some(b'\\') => {
                    let esc = match self.bump() {
                        Some(e) => e,
                        None => return Err(ParseError::Syntax),
                    };
                    match esc {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0C),
                        b'n' => out.push(0x0A),
                        b'r' => out.push(0x0D),
                        b't' => out.push(0x09),
                        b'u' => {
                            let hi = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                // High surrogate: a low surrogate must follow.
                                self.eat(b'\\')?;
                                self.eat(b'u')?;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return Err(ParseError::Syntax);
                                }
                                0x10000u32 + (((hi as u32) - 0xD800) << 10) + ((lo as u32) - 0xDC00)
                            } else if (0xDC00..0xE000).contains(&hi) {
                                return Err(ParseError::Syntax); // lone low surrogate
                            } else {
                                hi as u32
                            };
                            push_utf8(&mut out, cp);
                        }
                        _ => return Err(ParseError::Syntax),
                    }
                }
                Some(b) if b < 0x20 => return Err(ParseError::Syntax), // unescaped control
                Some(b) if b < 0x80 => out.push(b),
                Some(b) => {
                    // Multi-byte UTF-8: validate the sequence, then copy it.
                    let len = if b >= 0xF0 {
                        4
                    } else if b >= 0xE0 {
                        3
                    } else if b >= 0xC0 {
                        2
                    } else {
                        return Err(ParseError::Syntax);
                    };
                    let start = self.pos - 1;
                    let end = start.saturating_add(len);
                    if end > self.bytes.len() {
                        return Err(ParseError::Syntax);
                    }
                    if core::str::from_utf8(&self.bytes[start..end]).is_err() {
                        return Err(ParseError::Syntax);
                    }
                    let mut k = start;
                    while k < end {
                        out.push(self.bytes[k]);
                        k += 1;
                    }
                    self.pos = end;
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u16, ParseError> {
        let mut v: u16 = 0;
        let mut i = 0;
        while i < 4 {
            let b = match self.bump() {
                Some(b) => b,
                None => return Err(ParseError::Syntax),
            };
            let d = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return Err(ParseError::Syntax),
            };
            v = v.wrapping_mul(16).wrapping_add(d as u16);
            i += 1;
        }
        Ok(v)
    }
}

/// Decimal integer from ASCII bytes, without going through `str::parse`, so the
/// parser stays byte-oriented. The accumulator is checked against the
/// admissible range before multiplication; overflow returns IntegerOutOfRange.
fn parse_int(text: &[u8]) -> Result<i64, ParseError> {
    let mut i = 0usize;
    let negative = !text.is_empty() && text[0] == b'-';
    if negative {
        i = 1;
    }
    if i >= text.len() {
        return Err(ParseError::Syntax);
    }
    let mut acc: i64 = 0;
    while i < text.len() {
        let b = text[i];
        if !b.is_ascii_digit() {
            return Err(ParseError::Syntax);
        }
        let d = (b - b'0') as i64;
        // Bound the accumulator before the next multiply/add.
        // Saturating operations would be simpler but extract to checked binops,
        // which Aeneas cannot yet translate (AeneasVerif/aeneas#1257).
        if acc > (MAX_SAFE_INT - d) / 10 {
            return Err(ParseError::IntegerOutOfRange);
        }
        acc = acc * 10 + d;
        i += 1;
    }
    Ok(if negative { -acc } else { acc })
}

/// Encode one Unicode scalar as UTF-8.
fn push_utf8(out: &mut Vec<u8>, cp: u32) {
    if cp < 0x80 {
        out.push(cp as u8);
    } else if cp < 0x800 {
        out.push(0xC0 | ((cp >> 6) as u8));
        out.push(0x80 | ((cp & 0x3F) as u8));
    } else if cp < 0x10000 {
        out.push(0xE0 | ((cp >> 12) as u8));
        out.push(0x80 | (((cp >> 6) & 0x3F) as u8));
        out.push(0x80 | ((cp & 0x3F) as u8));
    } else {
        out.push(0xF0 | ((cp >> 18) as u8));
        out.push(0x80 | (((cp >> 12) & 0x3F) as u8));
        out.push(0x80 | (((cp >> 6) & 0x3F) as u8));
        out.push(0x80 | ((cp & 0x3F) as u8));
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;

    /// P3: a fractional part or exponent is rejected with the specific error, at
    /// top level and nested, rather than silently canonicalized.
    #[test]
    fn non_integer_numbers_are_rejected() {
        for input in ["1.5", "-0.1", "0.0", "1e3", "1E3", "1e-3"] {
            assert_eq!(parse(input), Err(ParseError::NonIntegerNumber), "{input}");
        }
        assert_eq!(parse(r#"{"a":2.5}"#), Err(ParseError::NonIntegerNumber));
        assert_eq!(parse("[1,2.5]"), Err(ParseError::NonIntegerNumber));
        assert_eq!(
            parse(r#"{"a":{"b":[0.5]}}"#),
            Err(ParseError::NonIntegerNumber)
        );
    }

    /// P3: duplicate member names are rejected, not resolved last-wins.
    #[test]
    fn duplicate_keys_are_rejected() {
        assert_eq!(parse(r#"{"a":1,"a":2}"#), Err(ParseError::DuplicateKey));
        assert_eq!(
            parse(r#"{"a":{"b":1,"b":2}}"#),
            Err(ParseError::DuplicateKey)
        );
    }

    #[test]
    fn safe_integer_range_is_enforced() {
        assert_eq!(parse("9007199254740991"), Ok(Value::Int(MAX_SAFE_INT)));
        assert_eq!(parse("-9007199254740991"), Ok(Value::Int(-MAX_SAFE_INT)));
        assert_eq!(
            parse("9007199254740992"),
            Err(ParseError::IntegerOutOfRange)
        );
        assert_eq!(
            parse("-9007199254740992"),
            Err(ParseError::IntegerOutOfRange)
        );
    }

    #[test]
    fn malformed_input_is_rejected() {
        assert_eq!(parse("{} {}"), Err(ParseError::TrailingContent));
        assert_eq!(parse("01"), Err(ParseError::Syntax));
        assert_eq!(parse(r#"{"a":1,}"#), Err(ParseError::Syntax));
        assert_eq!(parse("\"\u{1}\""), Err(ParseError::Syntax));
        assert_eq!(parse(r#""\ud800""#), Err(ParseError::Syntax));
    }

    /// Objects come out sorted regardless of input order, so canonicalization
    /// never has to sort.
    #[test]
    fn objects_are_sorted_on_construction() {
        match parse(r#"{"b":1,"a":2}"#).expect("parses") {
            Value::Object(members) => {
                let keys: Vec<&[u8]> = members.iter().map(|(k, _)| k.as_slice()).collect();
                assert_eq!(keys, [b"a".as_slice(), b"b".as_slice()]);
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn negative_zero_is_normalised() {
        assert_eq!(parse("-0"), Ok(Value::Int(0)));
    }

    /// Escaped surrogate pairs decode to the same bytes as the literal
    /// character, so both spellings hash identically.
    #[test]
    fn surrogate_pairs_and_literals_agree() {
        let escaped = parse(r#""\ud83d\udd10""#).expect("escaped parses");
        let literal = parse("\"\u{1F510}\"").expect("literal parses");
        assert_eq!(escaped, literal);
    }
}
