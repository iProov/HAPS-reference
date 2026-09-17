# Contributing

This crate exists so implementers have something concrete to disagree with. Conformance is defined by
the [specification](https://github.com/iProov/HAPS) and its test vectors, not by this code — if this
code disagrees with a normative input/expected-hash pair, investigate against the specification.
Illustrative evidence digests are not expected-hash test values.

## Before you start

Independent implementations in other languages are encouraged and are not second-class. If you are
writing one, the portable kit under `plugins/haps-interoperability/` and the language-neutral runner in
`conformance/` are meant for you; reports that the specification was ambiguous are especially useful.

## Checks that must pass

Use Rust with Clippy/rustfmt and Python 3.11+, as described in [the README](README.md#build-and-checks).

```sh
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all --check
python3 conformance/haps_conformance.py verify-lock
python3 scripts/sync_plugin_conformance.py --check
python3 scripts/check_readme_sync.py
python3 -m unittest discover -s conformance/tests
python3 -m unittest discover -s plugins/haps-interoperability/tests
cargo build --locked -p haps-reference-adapter
python3 conformance/haps_conformance.py test --adapter target/debug/haps-reference-adapter
python3 conformance/haps_conformance.py test --adapter plugins/haps-interoperability/examples/python/haps_adapter.py
```

The Kani harnesses and the Lean proofs are not part of that loop. Kani runs in CI; the Lean build does
not, and needs an Aeneas checkout outside this repository —
[docs/verification-toolchain.md](docs/verification-toolchain.md) has the pins and the traps.

## The restricted Rust subset

`haps-canon` is `no_std` with **zero dependencies** and is written to stay translatable by Charon and
Aeneas. That constrains what you may add to it: no `unsafe`, `async`, `dyn`, interior mutability or hash
maps. Existing code uses `Vec<u8>` and locally guarded indexing because particular extraction
experiments rejected other patterns. Those observations depend on the pinned toolchain and do not
justify unchecked indexing or arithmetic. Preserve bounds checks, input validation and typed errors;
see the toolchain notes for the exact experimental limits. If a change makes extraction fail, that is a
regression even though CI will not catch it.

## If you touch a vector or an identifier

The specification repository owns the vectors. Propose normative fixture or identifier changes there
first, with a rationale and expected values derived from the specification and independently checked
where possible. A failing test prints this implementation's output, not an independently correct answer;
do not copy it into a fixture merely to make the test pass.

After the specification change is committed, follow [test_vectors/SOURCE.md](test_vectors/SOURCE.md)
to refresh the vendored files and provenance, regenerate the lock, and synchronize the plugin assets.
Do not edit a vendored vector or lock to conceal a mismatch.

## House rules on claims

Claim less rather than more. Do not add wording that describes this crate as formally verified or
production-ready, and do not strengthen a claim about what is proved without the proof. If you find an
existing claim that overstates what holds, correcting it downward is a welcome contribution on its own.

## Code of Conduct and licensing

Participation is governed by [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). By contributing you agree your
contributions are licensed under this repository's licence (Apache-2.0; see [LICENSE](LICENSE)) on the
terms in its Section 5. Note the scope limits in [NOTICE](NOTICE).
