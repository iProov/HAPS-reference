//! JSON values for the experimental HAPS canonicalization core.
//!
//! The parser restricts numbers to integers in ±(2^53 − 1), produces UTF-8
//! strings and keys, and stores object members in canonical order without
//! duplicates. These are intended preconditions for canonicalization.
//!
//! The enum is public: directly constructed values can violate those conditions
//! and bypass the parser's nesting limit. `is_admissible` checks integer ranges
//! only; it is not a complete structural, UTF-8 or protocol validator.
//!
//! UTF-8 bytes and `Vec` keep the implementation accessible to extraction
//! experiments; their use is not a correctness proof.

use alloc::vec::Vec;

/// Largest integer magnitude permitted by §6.1: 2^53 − 1.
pub const MAX_SAFE_INT: i64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    /// An integer in `-MAX_SAFE_INT ..= MAX_SAFE_INT`, enforced at parse time.
    Int(i64),
    /// Unescaped UTF-8 bytes when produced by the parser; not checked here.
    Str(Vec<u8>),
    Array(Vec<Value>),
    /// Parser output has canonical key order and no duplicates. Direct
    /// construction must preserve these invariants; canonicalization does not sort.
    Object(Vec<(Vec<u8>, Value)>),
}

impl Value {
    /// Whether every integer in this value is within the admissible range.
    /// This does not check UTF-8, duplicate keys, key ordering, nesting depth,
    /// schemas or protocol semantics, even when it returns `true`.
    pub fn is_admissible(&self) -> bool {
        match self {
            Value::Null | Value::Bool(_) | Value::Str(_) => true,
            Value::Int(n) => *n >= -MAX_SAFE_INT && *n <= MAX_SAFE_INT,
            Value::Array(items) => {
                let mut i = 0;
                while i < items.len() {
                    match items.get(i) {
                        Some(v) => {
                            if !v.is_admissible() {
                                return false;
                            }
                        }
                        None => return false,
                    }
                    i += 1;
                }
                true
            }
            Value::Object(members) => {
                let mut i = 0;
                while i < members.len() {
                    match members.get(i) {
                        Some((_, v)) => {
                            if !v.is_admissible() {
                                return false;
                            }
                        }
                        None => return false,
                    }
                    i += 1;
                }
                true
            }
        }
    }
}
