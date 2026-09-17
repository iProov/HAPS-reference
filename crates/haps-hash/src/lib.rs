//! Experimental hashing and `sha256:` encoding for HAPS intent/presentation bytes.
//!
//! This is a partial reference implementation and is not production-ready.
//! SHA-256 uses `sha2`, whose implementation and cryptographic assumptions are
//! outside the verification experiments. The Kani harness covers only the
//! base64url alphabet lookup, not hashing or the complete encoder.
//!
//! A matching hash binds bytes under those assumptions; it does not establish
//! correct display, human presence, approval or protocol conformance.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};

use haps_canon::{canonicalize, Value};

const B64URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// base64url without padding, per the HAPS hash encoding.
pub fn base64url_unpadded(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes.get(i).copied().unwrap_or(0);
        let b1 = bytes.get(i + 1).copied();
        let b2 = bytes.get(i + 2).copied();
        push_b64(&mut out, (b0 >> 2) & 0x3F);
        match (b1, b2) {
            (Some(b1), Some(b2)) => {
                push_b64(&mut out, ((b0 << 4) | (b1 >> 4)) & 0x3F);
                push_b64(&mut out, ((b1 << 2) | (b2 >> 6)) & 0x3F);
                push_b64(&mut out, b2 & 0x3F);
            }
            (Some(b1), None) => {
                push_b64(&mut out, ((b0 << 4) | (b1 >> 4)) & 0x3F);
                push_b64(&mut out, (b1 << 2) & 0x3F);
            }
            (None, _) => {
                push_b64(&mut out, (b0 << 4) & 0x3F);
            }
        }
        i += 3;
    }
    out
}

/// Allocation-free core: map a 6-bit group to its base64url character.
///
/// Total for every `u8`: values above 63 have no encoding and return `None`
/// rather than indexing out of bounds. Kept separate from the string building so
/// it can be verified over all inputs without the model checker having to reason
/// about allocation.
#[allow(clippy::indexing_slicing)] // guarded by `six < 64`; the table is 64 bytes
pub const fn b64_char(six: u8) -> Option<u8> {
    if six < 64 {
        Some(B64URL[six as usize])
    } else {
        None
    }
}

fn push_b64(out: &mut String, six: u8) {
    if let Some(c) = b64_char(six) {
        out.push(c as char);
    }
}

/// `"sha256:" + base64url_unpadded(SHA-256(bytes))`.
pub fn sha256_prefixed(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::from("sha256:");
    out.push_str(&base64url_unpadded(digest.as_slice()));
    out
}

/// Hash a value satisfying `haps_canon::canonicalize`'s preconditions.
/// Does not validate a schema, derive a Signing View, or check human approval.
pub fn hash_value(value: &Value) -> String {
    let bytes: Vec<u8> = canonicalize(value);
    sha256_prefixed(bytes.as_slice())
}

/// Parse restricted JSON, then hash its canonical bytes.
///
/// Returns the `sha256:`-prefixed hash or a syntax/domain error. This is only
/// one part of processing an AI-INTENT: no schema, signature, freshness,
/// replay-state, display or human-approval check is performed.
pub fn parse_and_hash(input: &str) -> Result<String, haps_canon::ParseError> {
    let value = haps_canon::parse(input)?;
    Ok(hash_value(&value))
}

#[cfg(kani)]
mod proofs {
    use super::b64_char;

    /// Check alphabet lookup over every `u8`: accepted values map to an ASCII
    /// alphanumeric, `-` or `_`; values above 63 return None. This does not
    /// establish P17 for the full hash encoding or hashing implementation.
    #[kani::proof]
    fn base64url_alphabet_is_total_and_url_safe() {
        let six: u8 = kani::any();
        match b64_char(six) {
            None => assert!(six >= 64),
            Some(c) => {
                assert!(six < 64);
                assert!(c != b'=' && c != b'+' && c != b'/');
                let alnum = c.is_ascii_alphanumeric();
                assert!(alnum || c == b'-' || c == b'_');
            }
        }
    }
}
