# Security policy

This is an **experimental, partial** reference implementation of an experimental draft specification.
It is not production-ready. What is and is not established is set out in the [README](README.md)
and in the specification's
[limitations document](https://github.com/iProov/HAPS/blob/main/docs/limitations.md).

## Reporting a vulnerability

Email **security@iproov.com**, marking the subject line `HAPS reference`. This address is monitored
and works regardless of repository visibility or GitHub account status. It is the primary route, and
the one to use if anything below is unavailable.

You may instead use GitHub's
[private vulnerability reporting form](https://github.com/iProov/HAPS-reference/security/advisories/new)
(**Security → Report a vulnerability**) when it is available. That form requires a public repository
with private vulnerability reporting enabled; maintainers enable and monitor it at publication.
Reports submitted through it are private. Please do not open a public issue, and do not report a
suspected vulnerability in a pull request.

We aim to acknowledge a report within five working days. If you receive no acknowledgement, please
resend rather than assuming the report was judged out of scope.

A reproducer is worth more than a description: a failing input for the canonicalizer, a Kani harness
that finds the panic, or a Lean snippet that exhibits the gap. We do not operate a bounty.

## In scope

- A pair of distinct parsed values that canonicalize to the same bytes, or one value that canonicalizes
  two ways — this breaks the binding the whole design rests on.
- A document the parser accepts that the specification requires it to reject, or vice versa: duplicate
  member names, and numbers carrying a fractional part or exponent.
- A panic, arithmetic overflow or out-of-bounds access reachable from untrusted input.
- Disagreement between this code and a published test vector, in either direction.
- An error in the two Lean theorems, or a reason the Aeneas extraction does not faithfully represent the
  Rust it was extracted from.

## Known limitations

These limitations are already documented:

- The verifier components are not implemented. There is no signature verification, replay store,
  consent UI or Signing View derivation pipeline here.
- The Lean proofs cover two properties of a single comparison step, not whole-string ordering.
- Kani provides four bounded harnesses, not crate-wide panic freedom. All four verify under the
  pinned Kani version, but `canonical_ordering_is_transitive` does not complete on a standard
  hosted runner, so its recorded result comes from a maintainer run rather than from CI.
- The Lean proofs are not reproducible from a clone alone and are not a CI gate. See
  [docs/verification-toolchain.md](docs/verification-toolchain.md).
- `sha2` is assumed correct and is not re-verified.

Concrete security consequences, missing limitations and misleading claims remain in scope.
Documenting a gap does not make its consequences harmless; include new evidence or impact when
reporting a known limitation.
