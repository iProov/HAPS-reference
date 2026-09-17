# HAPS v0.4 implementation contract

Implement the scoped adapter contract below and use the bundled suite for limited checks. HAPS is an
experimental draft specification with a partial reference implementation, not production-ready
software. The draft specification defines intended behavior; the pinned vectors supply test examples.
The Rust implementation is a comparison implementation and may contain bugs or omissions.

## Admissible JSON

- Consume UTF-8 JSON and reject malformed input or trailing content.
- Detect duplicate object member names before constructing a last-wins map.
- Reject every number token with a fractional part or exponent before canonicalization.
- Accept integer tokens only in `[-9007199254740991, 9007199254740991]`.
- Normalize integer `-0` to `0`.
- Apply a documented finite nesting bound and return a typed error rather than panic or recurse without
  limit.

Do not use a generic decoder that has already collapsed duplicate keys, converted all numbers to binary
floating point, or discarded whether exponent syntax appeared.

## Canonicalization and hashing

- Canonicalize the parsed value using the RFC 8785 string/object rules relevant to HAPS's restricted
  numeric domain.
- Sort object keys lexicographically by UTF-16 code units. This differs from UTF-8 byte and Unicode
  scalar ordering for the suite's `🔐` and `Ｚ` keys.
- Preserve non-ASCII text as UTF-8; escape JSON controls, quote, and backslash minimally.
- Serialize integers in plain decimal without exponent, leading plus, leading zero, or negative zero.
- Hash the exact canonical UTF-8 bytes with SHA-256, encode base64url without padding, and prefix
  `sha256:`.

## Adapter boundary

Implement `.haps/conformance/adapter-protocol-v1.md` as a small stdin/stdout executable. Keep protocol
JSON parsing separate from candidate HAPS document parsing. Protocol output is one NDJSON object on
stdout; logs and diagnostics belong on stderr.

For the listed inputs, the runner checks the hash of returned canonical bytes independently,
compares it to pinned hashes, checks canonical fixed points, reverses object insertion order, and
requires specified rejection codes. It does not exhaustively check this contract or all JSON inputs.

## Claim boundary

The suite samples requirements associated with P1–P3 and VEC-01–03. Passing it does not establish those
properties for arbitrary inputs, full protocol conformance, or that a person saw and approved the
correct action. It is not a Consent Credential verifier or issuer. Do not add signature, trust, replay,
factor, certification, identity, biometric, or policy behavior unless the user separately requests and
scopes that work. Intended security and approval outcomes depend on those implementations and their
deployment conditions.

The Python example checks its documented depth bound after JSON parsing; sufficiently deep input may
hit the decoder's recursion limit first and return `SYNTAX`. It has no overall input or memory bound.
The bundled suite does not test these limits, every error code, or persistent adapter sessions. The
runner does not sandbox adapters and checks response size only after collecting output. Treat both as
experimental development tools and document the target implementation's remaining limits.
