//! Bounded Kani harnesses for selected canonicalization helpers.
//!
//! Ordering assertions use arbitrary fixed-length byte arrays (two bytes for
//! antisymmetry, one for transitivity); they do not cover all valid strings.
//! Integer assertions use inputs in ±10^6 and check formatting structure and
//! bounds, not whether decoding the digits recovers the input value.
//!
//! No parser harness is included. Parser rejection cases have unit tests;
//! general parser, canonicalization and totality properties remain open proof
//! obligations. A successful harness run applies only to that run's assertions,
//! input assumptions and unwind bounds, not the crate or protocol as a whole.

use crate::canon::{int_digits, INT_BUF};
use crate::order::cmp_canonical;

// ---------------------------------------------------------------- arithmetic

/// Check sign/digit structure, zero representation and buffer bounds for
/// integer serialization within ±10^6, with unwind bound 10.
///
/// This does not establish P17 for the full admissible range or the complete
/// crate. Selected admissible-range boundaries have unit tests in `canon.rs`;
/// general serialization correctness remains an open proof obligation.
#[kani::proof]
#[kani::unwind(10)]
fn int_digits_is_total_over_a_bounded_range() {
    let n: i64 = kani::any();
    kani::assume(n >= -1_000_000 && n <= 1_000_000);
    let mut buf = [0u8; INT_BUF];
    let len = int_digits(n, &mut buf);
    assert!(len > 0 && len <= INT_BUF);
    // Exactly one sign character, and only when negative.
    if n < 0 {
        assert!(buf[0] == b'-');
        assert!(len >= 2);
    } else {
        assert!(buf[0] != b'-');
    }
    // Zero is exactly "0"; this assertion does not rule out `-0` for negative inputs.
    if n == 0 {
        assert!(len == 1 && buf[0] == b'0');
    }
    // Every emitted byte after any sign is an ASCII digit, and there is no
    // leading zero unless the value is zero.
    let start = if n < 0 { 1 } else { 0 };
    assert!(buf[start] != b'0' || len == start + 1);
    let mut i = start;
    while i < len {
        assert!(buf[i] >= b'0' && buf[i] <= b'9');
        i += 1;
    }
}

// ---------------------------------------------------------------- ordering

/// Check reversal symmetry on arbitrary two-byte arrays, with unwind bound 4.
/// This does not check equality implies byte equality, or the whole-string order.
#[kani::proof]
#[kani::unwind(4)]
fn canonical_ordering_is_antisymmetric() {
    let a: [u8; 2] = kani::any();
    let b: [u8; 2] = kani::any();
    let ab = cmp_canonical(&a, &b);
    let ba = cmp_canonical(&b, &a);
    assert!(ab == ba.reverse());
}

/// Check transitivity on arbitrary one-byte arrays, with unwind bound 4.
#[kani::proof]
#[kani::unwind(4)]
fn canonical_ordering_is_transitive() {
    let a: [u8; 1] = kani::any();
    let b: [u8; 1] = kani::any();
    let c: [u8; 1] = kani::any();
    if cmp_canonical(&a, &b).is_le() && cmp_canonical(&b, &c).is_le() {
        assert!(cmp_canonical(&a, &c).is_le());
    }
}
