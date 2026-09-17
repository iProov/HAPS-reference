//! Canonical key ordering, over UTF-8 bytes.
//!
//! RFC 8785 orders object members by their **UTF-16 code units**, which is not
//! UTF-8 byte order. The two disagree whenever one key contains a character
//! above the BMP (a surrogate pair, leading unit in 0xD800..0xDC00) and another
//! contains a character in 0xE000..=0xFFFF: by code unit the surrogate sorts
//! first, by byte it sorts last. The published `canonicalization-edge-cases`
//! vector exercises exactly this.
//!
//! Written as two passes — decode to code units, then compare — rather than as
//! two lazy cursors. Two simultaneous `&mut` cursors matched as a tuple is a
//! shape Aeneas cannot currently translate (it reports an internal error); a
//! decode-then-compare pass supports extraction. It allocates intermediate
//! vectors; complete ordering and decoder proofs remain open.

use alloc::vec::Vec;
use core::cmp::Ordering;

// `slice[i]` rather than `slice.get(i)` inside these loops: Aeneas aborts
// translation on `<[T]>::get` in a loop (AeneasVerif/aeneas#1319), and this
// code is intended for extraction. Local bounds checks guard indexing;
// Kani ordering harnesses cover only fixed one- and two-byte arrays, not every
// path or input size. There is no general panic-freedom proof.

/// Decode UTF-8 bytes to UTF-16 code units.
///
/// Intended for valid UTF-8, as produced by the parser. This public helper
/// consumes malformed byte sequences without validating or rejecting them;
/// callers must not use it as a UTF-8 validator. It allocates an output vector.
pub fn utf16_units(bytes: &[u8]) -> Vec<u16> {
    let mut out: Vec<u16> = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let b0 = bytes[i];
        if b0 < 0x80 {
            out.push(b0 as u16);
            i += 1;
            continue;
        }
        let len = if b0 >= 0xF0 {
            4
        } else if b0 >= 0xE0 {
            3
        } else if b0 >= 0xC0 {
            2
        } else {
            // Stray continuation byte: consume it so the loop terminates.
            out.push(b0 as u16);
            i += 1;
            continue;
        };
        let mut cp: u32 = if len == 2 {
            (b0 & 0x1F) as u32
        } else if len == 3 {
            (b0 & 0x0F) as u32
        } else {
            (b0 & 0x07) as u32
        };
        let mut k = 1usize;
        while k < len && i + k < bytes.len() {
            cp = (cp << 6) | ((bytes[i + k] & 0x3F) as u32);
            k += 1;
        }
        if cp < 0x10000 {
            out.push(cp as u16);
        } else {
            let v = cp - 0x10000;
            out.push(0xD800 + ((v >> 10) as u16));
            out.push(0xDC00 + ((v & 0x3FF) as u16));
        }
        i += len;
    }
    out
}

/// Compare two valid UTF-8 keys by UTF-16 code unit, as RFC 8785 requires.
/// This helper assumes valid UTF-8 and does not validate its input.
pub fn cmp_canonical(a: &[u8], b: &[u8]) -> Ordering {
    let ua = utf16_units(a);
    let ub = utf16_units(b);
    let n = if ua.len() < ub.len() {
        ua.len()
    } else {
        ub.len()
    };
    let mut i = 0usize;
    while i < n {
        let x = ua[i];
        let y = ub[i];
        if x != y {
            return if x < y {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        i += 1;
    }
    if ua.len() == ub.len() {
        Ordering::Equal
    } else if ua.len() < ub.len() {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}
