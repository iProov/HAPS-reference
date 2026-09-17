---
name: haps-integrate
description: "Port or integrate HAPS v0.4 canonicalization and hashing in an existing software project, expose the language-neutral adapter, and test it with pinned vectors. Use when asked to add HAPS support, create a reference port in another language, or implement the HAPS conformance adapter. This experimental workflow does not establish full protocol conformance, correct display, human approval, or production readiness."
---

# Integrate HAPS

HAPS is an experimental draft specification with a partial reference implementation. It is not
production-ready. Passing the supplied tests does not establish full protocol conformance or that a
person saw and approved the correct action.

Create a native implementation that is reviewable in the target project's idioms, then use the
deterministic interoperability kit to report results for its finite test cases.

## Before implementation

1. Read the target repository's instructions, dependency policy, build system, tests, CI, and dirty
   state. Preserve unrelated work.
2. Read [references/implementation-contract.md](references/implementation-contract.md). Its parser and
   ordering requirements are load-bearing; ordinary JSON decoding often violates them.
3. Identify the requested scope. Unless the user explicitly asks for more, implement only strict
   parsing, canonicalization, hashing, stable rejection codes, and the adapter protocol.
4. Install the pinned kit when the project does not already contain it. Resolve this skill's plugin
   root, then run `<plugin-root>/bin/haps init <repo-root>`. This command installs only the locked
   runner, protocol, and vectors; it deliberately does not generate a language implementation.

## Implementation outcome

- Put the core behind a small native API and keep the adapter as a thin executable boundary.
- Reject duplicate member names before a normal last-wins object representation can erase them.
- Reject fractional and exponent number tokens before canonicalization, including values such as
  `1.0` or `1e2` that a decoder might normalize to integers.
- Sort object keys by UTF-16 code units, not UTF-8 bytes, Unicode scalar value, locale, or insertion
  order.
- Emit canonical UTF-8 bytes and compute the `sha256:` digest from those exact bytes.
- Implement `haps-conformance-adapter-v1` exactly. Send diagnostics to stderr and protocol responses to
  stdout.

Prefer the target project's existing trustworthy dependencies when they expose the required behavior.
If its JSON parser cannot preserve duplicate keys or number token syntax, add a strict pre-pass or use a
parser that can; do not patch over information after it has been lost.

For a Python target only, the plugin's `examples/python/haps_adapter.py` is a tested dependency-free
example. Read it as a starting point, then adapt its public API and packaging to the target project. Do
not mechanically translate it into other languages whose JSON and Unicode semantics differ.

## Test the port

Run the project's own tests first, then:

```sh
python3 .haps/conformance/haps_conformance.py verify-lock
python3 .haps/conformance/haps_conformance.py test \
  --adapter <built-adapter> \
  --json-report <artifact-dir>/haps-conformance.json \
  --junit-report <artifact-dir>/haps-conformance.xml
```

Add the same deterministic build and test commands to the existing CI system. For CI-specific guidance,
use the plugin's `haps-conformance` skill rather than inventing a second gate.

Report native unit tests, the pinned suite result, and CI state separately. A passing suite result
means only agreement with the tested inputs. Never call it HAPS-PP-0.4, HAPS-RP-0.4, PoHP certification,
formal verification, proof of human approval, or a guarantee against failure or fraud. Describe
security and approval as intended outcomes that depend on implementation and deployment conditions
beyond this port and its tests.
