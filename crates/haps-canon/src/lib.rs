//! Experimental partial canonicalization and parsing implementation for HAPS v0.4.0.
//!
//! This crate is not production-ready. Its tests and bounded harnesses do not
//! establish full protocol conformance, general panic freedom, or that a person
//! saw and approved the correct action. It implements a restricted JSON core,
//! not schemas, display, credential verification or an approval protocol.
//!
//! The dependency-free, byte-oriented implementation supports experiments with
//! Charon/Aeneas extraction. Only selected helpers have Kani harnesses and only
//! two extracted comparison-step properties have Lean theorem bodies. Complete
//! parser, canonicalizer and totality proofs remain open; P1, P3 and P17 in
//! `verifiable-properties-v0.1.md` are targets, not completed crate-wide results.
//!
//! Public [`Value`] constructors can bypass the parser's invariants. Read the
//! preconditions of [`canonicalize`] and apply input and resource limits in the
//! caller. Hashing is separate in `haps-hash`, whose SHA-256 implementation is
//! outside these proof experiments.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod canon;
pub mod order;
pub mod parse;
pub mod value;

pub use canon::{canonicalize, int_digits, INT_BUF};
pub use order::cmp_canonical;
pub use parse::{parse, ParseError, MAX_DEPTH};
pub use value::{Value, MAX_SAFE_INT};

#[cfg(kani)]
mod proofs;
