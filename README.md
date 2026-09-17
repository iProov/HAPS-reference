# HAPS partial reference implementation

**This is an experimental draft specification and partial reference implementation. It is not
production-ready.** This repository implements a small Rust canonicalization and hashing core for
[HAPS v0.4.0](https://github.com/iProov/HAPS/blob/main/specification/draft/haps-v0.4.0.md),
with an interoperability kit and limited verification experiments.

**Passing the supplied tests does not establish full protocol conformance or that a person saw and
approved the correct action.** The specification is intended to support evidence of human presence
and approval bound to an action, subject to implementation and deployment conditions. This code does
not implement or validate the display, human interaction, or execution of that action, and provides no
guarantee against fraud.

The specification lives in **iProov/HAPS**; this repository is **iProov/HAPS-reference**. The Rust
crate names and the `haps` command use the HAPS name. The interoperability kit is
language-neutral; Rust is not a requirement for other implementations.

## Implementation scope and limitations

The workspace contains `haps-canon`, `haps-hash`, and `haps-reference-adapter`. It does not yet contain
`haps-core` or `haps-verify`, a Presence Provider, or a Relying Party implementation.

- **Only JSON canonicalization and hashing are implemented.** The parser checks the restricted JSON
  value domain, not AI-INTENT schemas, profile semantics, or permitted actions. There is no Signing
  View derivation or renderer, credential/signature validation, holder binding, assurance policy,
  identity verification, nonce/replay state, or atomic action execution here.
- **Human approval depends on the deployment.** Correct display and explicit approval require a
  trustworthy provider, renderer and device, appropriate presence factors, protected keys, and a
  relying party that checks the bindings and executes the matching action. A matching hash binds
  bytes; it does not establish what appeared on a screen, who interacted with it, or what they
  understood or were authorized to do. These conditions are outside this implementation and its tests.
- **Public values can bypass parser invariants.** `Value` is publicly constructible. `canonicalize`
  and `hash_value` assume valid UTF-8, unique object keys already in canonical order, admissible
  integers, and suitable nesting. They do not validate these preconditions or sort manually built
  objects. `Value::is_admissible` checks integer ranges only. Use `parse` / `parse_and_hash` for input
  documents, while separately validating the required schema and application semantics.
- **Resource exhaustion and failure remain possible.** The parser has a nesting limit of 64 but no
  input-byte or collection-size limit; parsing, sorting and serialization allocate memory and may be
  expensive. Directly constructed values can also bypass that nesting limit. The adapter reads
  unbounded input lines. There is no general proof of termination, panic freedom, allocation success,
  or resilience to adversarial workload. Errors returned as `Result` do not cover every runtime failure.
- **Some exported helpers have narrower intended domains than their types.** `int_digits` is intended
  for integers within ±(2^53 − 1); it does not reject out-of-range arguments and its saturating
  negation does not serialize `i64::MIN` faithfully. Key-order helpers expect valid UTF-8 and do not
  serve as UTF-8 validators.

These limitations are documented here without changing the protocol or redesigning the implementation.

## Tests and verification scope

The inventory below describes checked-in tests, harnesses and theorem bodies, not a blanket result for
this revision. A result must identify its source revision, toolchain, assertions and input bounds.

**Kani — 4 harnesses** (`cargo kani -p haps-canon` and `cargo kani -p haps-hash`):

| Harness | Assertions | Input domain |
| --- | --- | --- |
| `canonical_ordering_is_antisymmetric` | `cmp(a,b) = cmp(b,a).reverse()` | pairs of arbitrary 2-byte arrays, including invalid UTF-8; unwind bound 4 |
| `canonical_ordering_is_transitive` | `a ≤ b ∧ b ≤ c ⟹ a ≤ c` | triples of arbitrary 1-byte arrays; unwind bound 4 |
| `base64url_alphabet_is_total_and_url_safe` | values 0–63 map to an ASCII alphanumeric, `-` or `_`; values above 63 return `None` | every `u8` |
| `int_digits_is_total_over_a_bounded_range` | sign placement, zero encoding, no leading zero, ASCII digits, and buffer bounds | integers in ±10⁶; unwind bound 10 |

The ordering harnesses do not establish a total order over arbitrary-length strings or that a comparison
result of equality implies byte equality. The integer harness does not prove that the digits represent
the input value. The alphabet harness does not cover the complete base64url encoder or SHA-256.
No Kani harness targets the parser. Earlier parser experiments did not finish in practical time; tests
cover selected rejection cases, while general parser and serializer correctness remain open obligations.

**Recorded result.** All four harnesses verify with Kani 0.67.0 — the version CI pins — and CBMC
6.11.0. The figures below are from a maintainer run on an Apple M3 (arm64, 16 GB) against this
revision, with wall-clock times and the peak resident set for the longest harness recorded alongside:

| Harness | Properties | Result | Wall |
| --- | --- | --- | --- |
| `canonical_ordering_is_antisymmetric` | 0 of 356 failed (10 unreachable) | SUCCESSFUL | 206 s |
| `canonical_ordering_is_transitive` | 0 of 345 failed (14 unreachable) | SUCCESSFUL | 657 s |
| `int_digits_is_total_over_a_bounded_range` | 0 of 65 failed | SUCCESSFUL | 3 s |
| `base64url_alphabet_is_total_and_url_safe` | 0 of 49 failed | SUCCESSFUL | 2 s |

**Three of these run in CI; `canonical_ordering_is_transitive` is not run there.** Across four
attempts on a standard hosted runner, that job was terminated during solving without ever producing a
verdict — after roughly two, two, seventeen and five minutes, reported variously as failure (SIGTERM,
exit 143) and as cancellation. Symbolic execution completes each time and CBMC emits 14,468
verification conditions after simplification; the run dies in the solving phase that follows. Peak
memory for the maintainer run was 2.9 GB against the runner's 7.8 GB, so memory exhaustion is not a
supported explanation, and the cause has not been established.

Because that job cannot yield a trustworthy signal, it is excluded from CI rather than left to fail,
and the maintainer run above is its authoritative record. **This is a known gap in automated
coverage:** transitivity of the canonical ordering is checked only when a maintainer runs the harness
and records the result, which must be redone on the release revision before any release. It is not
evidence that the harness passes in CI.

A terminated or cancelled run is neither a discovered counterexample nor a successful proof. Do not
record a result from one; rerun it and name the revision and tool versions, as above.

Property counts are tied to the harnesses, the Kani version and the unwind bounds, and can legitimately
differ on another toolchain — Kani 0.68.0, for example, reports 332 properties for the transitive
harness rather than 345, and takes markedly longer on `canonical_ordering_is_antisymmetric`. Reproduce
individually with `cargo kani -p haps-canon --harness <name>` and
`cargo kani -p haps-hash --harness <name>`.

These results are bounded by the input domains and unwind bounds in the table above and say nothing
beyond them. They are not panic freedom for either crate: these are four decidable checks over small
constrained inputs. Reading the output, the `Description: "assertion failed: ..."` lines under each
harness enumerate the properties Kani checked, not failures; the verdict is the `0 of N failed` tally
and the `VERIFICATION:- SUCCESSFUL` line beneath them.

**Core tests — 20, in the source inventory:** VEC-01/02/03, fixed points for supplied vectors,
UTF-16 versus UTF-8 ordering, string escaping, duplicate-key and non-integer rejection, and selected
admissible-range boundaries. The adapter adds four protocol tests, so
`cargo test --workspace --all-targets` currently runs 24 tests. Passing these cases does not prove P1–P3
for all inputs or establish P17 (totality).

**Aeneas → Lean 4: two step-level theorem bodies.**
[`verification/Proofs/Ordering.lean`](verification/Proofs/Ordering.lean) contains:

- `cmp_loop_body_antisymm`: if a comparison step returns `ok (done o)`, swapping its arguments
  returns the swapped ordering;
- `cmp_loop_body_irrefl`: if a step comparing a vector with itself returns `ok (done o)`, `o` is equality.

Both require a successful terminating step as a hypothesis. They do not establish termination,
absence of failures, UTF-8 decoding correctness, the whole comparison loop, or canonicalization.
The source has no `sorry` placeholders, but that alone is not evidence that the current Rust is verified.
The Lean project depends on an external Aeneas checkout; CI runs neither extraction nor `lake build`
and does not check correspondence between the committed Lean and current Rust. See
[the toolchain notes](docs/verification-toolchain.md) for setup and remaining gaps.

Property identifiers refer to
[`verifiable-properties-v0.1.md`](https://github.com/iProov/HAPS/blob/main/specification/draft/verifiable-properties-v0.1.md),
whose document version remains v0.1 and applies to HAPS v0.4.x. These are target properties, not a list
of completed implementation proofs. The verification experiments rely on the models and correctness
of Kani/CBMC, Charon/Aeneas and Lean, their toolchains, and relevant runtime assumptions. SHA-256 uses
`sha2`; its implementation and cryptographic assumptions are outside these proofs.

## Layout

```text
crates/haps-canon     Restricted JSON parsing and RFC 8785 canonicalization;
                      no_std + alloc, with no library dependencies.
crates/haps-hash      SHA-256 and sha256: encoding; depends on haps-canon and sha2.
crates/haps-reference-adapter
                      NDJSON executable for the portable adapter protocol;
                      depends on the core crates and serde_json.
conformance/          Python runner, protocol, pinned suite, provenance lock,
                      reports, and negative-control tests.
plugins/haps-interoperability
                      Distributable Codex plugin with a self-contained kit copy.
haps                  Source-checkout entry point for haps init.
verification/         Lean project: Order/ is generated output;
                      Proofs/ contains hand-written theorems about that output.
test_vectors/         Vendored vectors, with provenance in SOURCE.md.
scripts/sync-vectors.sh
                      Maintainer refresh helper that overwrites vendored vectors.
scripts/check_readme_sync.py
                      CI check of README package, test, harness, skill and suite inventories.
```

The crates forbid `unsafe` in their own source. `haps-canon` and the adapter deny explicit `unwrap`,
`expect` and `panic` uses through Clippy. `haps-canon` allows indexing for extraction compatibility;
these lints and coding choices do not establish panic freedom or apply to all dependency code.

## Build and checks

Run these from the repository root with Rust and Python 3.11+ installed (the README check uses
Python's `tomllib`). Clippy and rustfmt must be installed for the selected Rust toolchain:

```sh
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all --check
python3 scripts/check_readme_sync.py
python3 conformance/haps_conformance.py verify-lock
```

Optional verification experiments require the separate setup in
[docs/verification-toolchain.md](docs/verification-toolchain.md):

```sh
cargo kani -p haps-canon
cargo kani -p haps-hash
(cd verification && lake build)
```

For a read-only comparison with a sibling specification checkout:

```sh
diff -ru ../HAPS/test_vectors/v0.4 test_vectors/v0.4
```

`scripts/sync-vectors.sh ../HAPS` is a **mutating maintainer refresh**, not a drift-only check.
A vector upgrade also requires reviewing provenance and the locked kit; see
[conformance/README.md](conformance/README.md).

## Interoperability kit

An implementation in any language can expose the NDJSON executable described in
[`conformance/adapter-protocol-v1.md`](conformance/adapter-protocol-v1.md). The runner checks pinned
specification/vector provenance, independently hashes returned canonical bytes, checks selected fixed
points and insertion-order variants, and writes JSON and JUnit reports.

Initialize the kit in another repository without generating implementation code:

```sh
# From an HAPS-reference checkout:
./haps init /path/to/your-project
```

Alternatively, put `HAPS-reference/plugins/haps-interoperability/bin` on `PATH` and run `haps init`.
With no target, `haps init` uses the current directory. It creates `.haps/conformance` with the locked
runner, protocol, vectors, and provenance data. Readers choose their language and write the adapter.
The command is idempotent, refuses to overwrite a differing kit, and reserves `--update` for an explicit
suite upgrade. `haps check [target]` detects installed-kit drift.

```sh
cargo build --locked -p haps-reference-adapter
python3 conformance/haps_conformance.py test \
  --adapter target/debug/haps-reference-adapter \
  --json-report target/haps-conformance.json \
  --junit-report target/haps-conformance.xml
```

The pinned suite currently executes 12 checks. CI runs the suite against both the Rust adapter and the
independent Python example in the plugin. Runner tests include a deliberately naïve JSON negative
control to exercise detection of Unicode-ordering and malformed-input mismatches.

The distributable Codex plugin at [`plugins/haps-interoperability`](plugins/haps-interoperability)
contains two skills:

- `haps-integrate` ports the narrow canonicalization/hash core into a project's native language and
  implements the adapter;
- `haps-conformance` installs the pinned offline kit, diagnoses mismatches, and adds the same checks
  to the project's CI.

A passing suite result covers only its VEC-01–03 and selected P1–P3 cases. It does not establish
HAPS-PP-0.4 or HAPS-RP-0.4 conformance, PoHP certification, signature verification, replay safety,
correct human display/approval, or production readiness. The draft
[conformance requirements](https://github.com/iProov/HAPS/blob/main/specification/draft/conformance-v0.4.md)
have a broader scope than this suite.

## Participation

- [CONTRIBUTING.md](CONTRIBUTING.md) — the checks that must pass, and the restricted Rust subset that
  supports the extraction experiments.
- [SECURITY.md](SECURITY.md) — how to report a vulnerability privately, and the known gaps that are
  already documented.
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) — Contributor Covenant v2.1.

Report disagreements with the normative input/expected-hash pairs. Illustrative policy and evidence
objects are not complete verification tests.

## License

Licensed under the [Apache License, Version 2.0](LICENSE). The following notice is also provided in
[NOTICE](NOTICE).

Scope of this License:

This repository, comprising the Human Approval and Presence Specification (HAPS) specification, reference application, and accompanying materials (the “Work”) is licensed under the Apache License, Version 2.0. Use of the Work under the License grants no rights to any patent, copyright, trade secret or other intellectual property right in iProov’s proprietary liveness technology, models, or services.

For clarity, while iProov welcomes your use of the HAPS specification and procedure in accordance with the License, including as incorporated into third-party products and services, use of the iProov name or branding (including trade marks) beyond identifying the origin of this Work, including any suggestion of endorsement, certification, or partnership, is not licensed (see Apache License 2.0, Section 6).
