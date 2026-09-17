#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::indexing_slicing
)]

//! Selected canonicalization/hash checks against published vectors.
//! Passing these cases does not establish full protocol conformance.
//!
//! VEC-01: reproduce the published input/expected-hash pairs; illustrative evidence digests are excluded.
//! VEC-02: pass the canonicalization edge-case vector.
//! VEC-03: reject the negative vector rather than hashing it.

use std::fs;
use std::path::PathBuf;

use haps_canon::{canonicalize, cmp_canonical, parse, ParseError};
use haps_hash::hash_value;

fn vectors() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_vectors/v0.4")
        .canonicalize()
        .expect("vendored vectors present")
}

fn read(name: &str) -> String {
    fs::read_to_string(vectors().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn check_hash(doc: &str, expected_file: &str) {
    let value = parse(&read(doc)).unwrap_or_else(|e| panic!("{doc} must parse, got {e:?}"));
    let expected = read(expected_file).trim().to_string();
    assert_eq!(hash_value(&value), expected, "{doc} -> {expected_file}");
}

#[test]
fn vec01_intent_hash() {
    check_hash(
        "action-intent.payment.transfer.sample.json",
        "expected_intent_hash.txt",
    );
}

#[test]
fn vec01_presentation_hash() {
    check_hash(
        "signing-view.payment.transfer.sample.json",
        "expected_presentation_hash.txt",
    );
}

#[test]
fn vec02_canonicalization_edge_cases() {
    check_hash(
        "canonicalization-edge-cases.sample.json",
        "expected_canonicalization_edge_cases_hash.txt",
    );
}

#[test]
fn vec03_fractional_number_is_rejected() {
    let doc = read("invalid/action-intent.fractional-number.sample.json");
    assert_eq!(
        parse(&doc),
        Err(ParseError::NonIntegerNumber),
        "the negative vector must be rejected, not hashed"
    );
}

/// The discriminating property of the edge-case vector: a surrogate-pair key
/// sorts *before* a BMP key above 0xE000 by UTF-16 code unit, and after it by
/// UTF-8 byte order. A canonicalizer sorting by bytes produces a different hash,
/// which is what makes this vector able to detect a non-JCS implementation.
#[test]
fn utf16_ordering_differs_from_byte_ordering() {
    let surrogate = "\u{1F510}".as_bytes(); // 🔐, code units D83D DD10
    let bmp = "\u{FF3A}".as_bytes(); // Ｚ, code unit FF3A
    assert!(cmp_canonical(surrogate, bmp).is_lt(), "UTF-16 order");
    assert!(surrogate > bmp, "UTF-8 byte order is the opposite");
}

/// Every object in canonical output is emitted in canonical key order.
#[test]
fn canonical_output_is_ordered() {
    let value = parse(&read("canonicalization-edge-cases.sample.json")).expect("parses");
    let bytes = canonicalize(&value);
    let out = std::str::from_utf8(&bytes).expect("canonical output is valid UTF-8");
    let pos_surrogate = out.find("\u{1F510}").expect("surrogate key present");
    let pos_bmp = out.find("\u{FF3A}").expect("bmp key present");
    assert!(
        pos_surrogate < pos_bmp,
        "surrogate-pair key must precede the BMP key"
    );
}

/// Non-ASCII values are emitted literally, not as \\u escapes — the other way a
/// deterministic-JSON encoder diverges from JCS.
#[test]
fn non_ascii_is_not_escaped() {
    let value = parse(r#"{"a":"£250.75"}"#).expect("parses");
    assert_eq!(canonicalize(&value), "{\"a\":\"\u{a3}250.75\"}".as_bytes());
}

/// Canonical form is a fixed point: canonicalizing canonical output reproduces
/// it exactly (property P1, on real documents rather than bounded ones).
#[test]
fn canonical_form_is_a_fixed_point_on_every_vector() {
    for name in [
        "action-intent.payment.transfer.sample.json",
        "signing-view.payment.transfer.sample.json",
        "canonicalization-edge-cases.sample.json",
        "haps-challenge.sample.json",
        "consent-credential.webauthn-uv.sample.json",
        "consent-credential.otp.sample.json",
        "provider-certification.provider-b.sample.json",
        "haps-digital-credentials-portrait-liveness.sample.json",
    ] {
        let v1 = parse(&read(name)).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let c1 = canonicalize(&v1);
        let c1_str = std::str::from_utf8(&c1).expect("canonical output is valid UTF-8");
        let v2 = parse(c1_str).unwrap_or_else(|e| panic!("{name} canonical form: {e:?}"));
        assert_eq!(v1, v2, "{name}: round trip changed the value");
        assert_eq!(canonicalize(&v2), c1, "{name}: not a fixed point");
    }
}

/// The policy vectors carry the published hashes, so an RP checking binding
/// against the published Action Intent will match.
#[test]
fn policy_vectors_are_bound_to_the_published_hashes() {
    let intent = read("expected_intent_hash.txt").trim().to_string();
    let presentation = read("expected_presentation_hash.txt").trim().to_string();
    for name in [
        "consent-credential.webauthn-uv.sample.json",
        "consent-credential.otp.sample.json",
    ] {
        let doc = read(name);
        assert!(doc.contains(&intent), "{name} must carry intent_hash");
        assert!(
            doc.contains(&presentation),
            "{name} must carry presentation_hash"
        );
    }
}

#[test]
fn duplicate_keys_and_trailing_content_are_rejected() {
    assert_eq!(parse(r#"{"a":1,"a":2}"#), Err(ParseError::DuplicateKey));
    assert_eq!(parse("{} {}"), Err(ParseError::TrailingContent));
}
