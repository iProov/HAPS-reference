# Verification experiments and toolchain

This repository is a **partial reference implementation of an experimental draft specification**,
and is **not production-ready**. The tests and proof experiments described here cover selected
canonicalization helpers. Passing them does not establish full protocol conformance or that a person
saw and approved the correct action. See the [implementation limitations](../README.md#implementation-scope-and-limitations).

## Evidence and reproducibility limits

- The repository contains four Kani harnesses: three in `haps-canon` and one in `haps-hash`.
  CI runs one job per harness for the three it can complete. All four verify under Kani 0.67.0, but
  only those three complete on a standard hosted runner. `canonical_ordering_is_transitive` has never completed there: four attempts
  were terminated during solving without a verdict, after roughly 2, 2, 17 and 5 minutes, reported
  variously as failure (SIGTERM, exit 143) and as cancellation. It is therefore excluded from CI and
  verified by a recorded maintainer run instead. Treat that as a known gap in automated coverage, and
  rerun the harness on the release revision before any release.
- Reference measurement for that maintainer run, on an Apple M3 (arm64, 16 GB) with Kani 0.67.0 and
  CBMC 6.11.0: antisymmetric 206 s, transitive 657 s, int-digits 3 s, base64url 2 s. Peak resident
  set for the transitive harness was 2.9 GB. Because that is well under the hosted runner's 7.8 GiB,
  memory exhaustion is not a supported explanation for the CI termination and the actual cause has
  not been established. Do not restate a cause without evidence.
- Toolchain version materially changes these numbers. Kani 0.68.0 reports 332 properties for the
  transitive harness rather than 345, and takes over 30 minutes on
  `canonical_ordering_is_antisymmetric` against 206 s for 0.67.0. Keep the local version equal to
  the version CI pins when reproducing a recorded result.
  [The README](../README.md#tests-and-verification-scope) lists their exact assertions and bounds.
  There is no parser harness, whole-program proof, or general panic-freedom result.
- `verification/Proofs/Ordering.lean` contains two theorem bodies about one extracted comparison step.
  Both assume a successful terminating result (`ok (done o)`). They do not show that a step or loop
  always terminates successfully, or prove the parser, UTF-8 decoder, canonicalizer or hash function.
- `verification/lakefile.lean` requires `../../tools/aeneas/backends/lean`, a path **outside this
  repository**. A fresh clone cannot run `lake build` until that dependency is provided. The path
  itself does not enforce the commit pin documented below.
- CI runs Rust tests, Clippy, formatting, Kani harnesses and interoperability checks. It does not run
  Charon extraction or `lake build`, and does not check that committed `verification/Order/` output
  corresponds to current Rust source. A source change can therefore leave the Lean artifacts stale.

The checked-in theorem bodies have no `sorry` placeholders. Their existence, or a successful build of
committed Lean, is not a proof of the current Rust implementation. A reproducible verification claim
would need the source revision, extraction inputs/options, tool revisions, generated output and build
results, with the model assumptions and theorem hypotheses stated. Those checks are not automated here.

## Recorded toolchain

These pins describe the development environment used for the committed experiment, not a supported
platform matrix. Installation instructions below have not been tested as a fresh-machine bootstrap.

| Tool | Recorded version |
| --- | --- |
| Aeneas | `7161abf2b8a830d7534f007f2b82e7931e66b921` |
| Charon | `fc94dcc3162e4adb4a6a3094e8d2e5b9900d35ae`, as named by that Aeneas revision's `charon-pin` |
| Charon Rust toolchain | `nightly-2026-08-18`; components are listed in Charon's `rust-toolchain` |
| Lean | `leanprover/lean4:v4.31.0`, from `verification/lean-toolchain` |
| OCaml | 5.4.1 via opam |
| Kani | 0.67.0; CI pins this verifier version through the Kani action |

## Run Kani

Kani installation has two steps, including setup of its downloaded toolchain. See the
[official installation guide](https://model-checking.github.io/kani/install-guide.html) for supported
hosts and prerequisites. To use the recorded version, with Rust installed through rustup:

```sh
cargo install --locked kani-verifier --version 0.67.0
cargo kani setup
cargo kani -p haps-canon
cargo kani -p haps-hash
```

Run each command from the repository root. A success result applies to the assertions and domains of
the named harnesses, with that run's configuration; it does not extend to untested functions or inputs.
The ordering domains are fixed byte arrays, not arbitrary-length valid strings. The integer harness is
bounded to ±10⁶ and checks formatting structure, not numeric round-trip equality. The hash harness
covers only the alphabet lookup over `u8`, not the full encoder or cryptographic primitive.

## Build the committed Lean project

The expected directory layout is:

```text
parent/
  HAPS-reference/
    verification/
  tools/
    aeneas/
    charon/       # needed for regeneration, not for checking committed Lean alone
```

For a fresh dependency installation, run from the `HAPS-reference` root:

```sh
mkdir -p ../tools
git clone https://github.com/AeneasVerif/aeneas.git ../tools/aeneas
git -C ../tools/aeneas checkout 7161abf2b8a830d7534f007f2b82e7931e66b921
```

If the checkout already exists, inspect it and its revision before changing it. Install Lean's `elan`
toolchain manager using the [official Lean setup instructions](https://lean-lang.org/install/), then:

```sh
(cd verification && lake build)
```

The first build may download Lean and dependencies, including mathlib. This checks the committed
Lean definitions and proof bodies; it does not regenerate them or check their correspondence to Rust.

## Regeneration prerequisites

Regeneration additionally needs Charon and the Aeneas executable. The relevant pinned upstream
[Charon README](https://github.com/AeneasVerif/charon/blob/fc94dcc3162e4adb4a6a3094e8d2e5b9900d35ae/README.md)
and [Aeneas README](https://github.com/AeneasVerif/aeneas/blob/7161abf2b8a830d7534f007f2b82e7931e66b921/README.md)
describe their build prerequisites. Start Charon from its pinned revision, after selecting the Aeneas
revision above. Do not use `git checkout $(cat charon-pin)`: that file includes a comment as well as the
commit identifier.

```sh
# From the HAPS-reference root:
git clone https://github.com/AeneasVerif/charon.git ../tools/charon
git -C ../tools/charon checkout fc94dcc3162e4adb4a6a3094e8d2e5b9900d35ae
```

The recorded macOS build used opam with OCaml 5.4.1 and Homebrew GNU Make (`gmake`). Charon's
`make build-charon-rust` creates its executable under `bin/`; building and installing its OCaml packages
provides `charon-ml`. Aeneas needs the corresponding Charon checkout (or a `charon` symlink to it) and
its own OCaml dependencies before `gmake build`. Add the resulting Charon and Aeneas `bin` directories
to `PATH` if invoking them by name.

Two issues from the development experiment:

- **Rust toolchain selection:** a Homebrew `rustc` ahead of the rustup shim can ignore Charon's
  `rust-toolchain`, causing missing compiler-internal crates. Check `command -v rustc` and
  `rustc --version` inside Charon; use the pinned rustup toolchain.
- **Extraction preset:** Aeneas expects LLBC generated with `charon cargo --preset=aeneas`.
  Run Charon inside the crate being extracted, then pass that output to
  `aeneas -backend lean -dest <output-directory> <input.llbc>`.

These are upstream command forms, not an automated recipe for reproducing the committed `Order`
module. This repository has no checked-in regeneration script recording its complete selection and
output options. Reproducing that module and comparing it to current Rust remains a manual task.

## Implementation choices and open work

Earlier extraction experiments encountered expensive dependency traversal and unsupported Rust
patterns. `haps-canon` is now dependency-free; `haps-hash` holds the `sha2` dependency, and the adapter
uses `serde_json`. Claims that the canonicalizer still depends on `sha2` or must be split are obsolete.
The core now uses UTF-8 bytes and `Vec<u8>` to make more operations accessible to extraction. A
successful translation supplies definitions to reason about; it does not establish their correctness.

| Recorded limitation | Current implementation choice and remaining limit |
| --- | --- |
| Slice `get` inside a loop triggered an extraction error ([Aeneas #1319](https://github.com/AeneasVerif/aeneas/issues/1319)) | `haps-canon` allows indexing with local bounds checks. Kani covers only the named harness domains, not every indexing path or all input sizes. |
| Checked/saturating arithmetic caused translation failures ([Aeneas #1257](https://github.com/AeneasVerif/aeneas/issues/1257)) | `parse_int` checks its accumulator against the admissible integer range before multiplication. This is an implementation check, not a completed parser proof. |
| Unsupported loop control in parser extraction | Some parser loops were restructured. Complete parser extraction and proofs remain unfinished. |
| String operations extracted as uninterpreted external definitions | Byte-oriented code reduces that obstacle; external models, allocation behavior and translation correctness still require scrutiny. |

Remaining work includes proving the complete comparison loop and UTF-8 conversion, extending to
canonicalization and parsing, documenting all model assumptions, and automating pinned extraction,
source correspondence and Lean checks. None of those steps would by itself establish correct human
display or approval, which require separate implementation and deployment evidence.
