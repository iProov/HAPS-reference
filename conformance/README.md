# HAPS interoperability kit

This directory contains a deterministic, language-neutral runner for the HAPS v0.4 canonicalization
and hashing surface currently implemented by this repository. HAPS is an experimental draft
specification with a partial reference implementation. This kit is not production-ready. Passing its
tests does not establish full protocol conformance or that a person saw and approved the correct
action.

## Run the reference adapter

```sh
python3 conformance/haps_conformance.py verify-lock
cargo build -p haps-reference-adapter
python3 conformance/haps_conformance.py test \
  --adapter target/debug/haps-reference-adapter \
  --json-report target/haps-conformance.json \
  --junit-report target/haps-conformance.xml
```

To run this limited suite against another language, provide an executable adapter implementing
[`adapter-protocol-v1.md`](adapter-protocol-v1.md). The runner never invokes a shell and accepts adapter
arguments through repeated `--adapter-arg` options.

The lock records the source specification commit and pins the suite, protocol, runner, and vector
bytes. `verify-lock` checks those local file digests against the lock; it does not authenticate the
upstream repository or prove the recorded provenance. Regenerate it with
`python3 conformance/update_lock.py` only for an intentional, reviewed kit/specification update; never
rewrite the lock to make a failing implementation pass. Inspect the diff, retain accurate source
provenance, and sync the portable plugin assets with `python3 scripts/sync_plugin_conformance.py --write`.

The standalone plugin and its portable kit include byte-identical copies of the repository's
[`LICENSE`](../LICENSE) and [`NOTICE`](../NOTICE). `haps init` carries both into the target repository's
`.haps/conformance` directory. `scripts/sync_plugin_conformance.py --check` checks both plugin copies
against the originals; `haps check` also detects missing or changed licensing files in an installed
kit. The conformance lock checks the test materials, not these licensing files.

## Result meaning

A passing result means the adapter matched the expectations for the finite inputs in
[`suite-v0.4.json`](suite-v0.4.json). The current suite has nine cases that expand to twelve executions:

- Three positive documents, each in source and reversed-key compact form, with expected hashes and
  canonical fixed-point checks.
- Six rejection examples: fractional and exponent numbers, duplicate keys, an out-of-range integer,
  trailing content, and malformed syntax.

These are examples associated with P1–P3 and VEC-01–03, not proofs of those properties for arbitrary
input. The suite does not exercise every adapter requirement, nesting limit, Unicode edge case, or
resource-exhaustion condition. It does not establish signature verification, issuer trust,
audience/freshness policy, replay prevention, provider certification, identity binding, any PoHP level,
or full Presence Provider/Relying Party conformance. It cannot establish correct display, human
understanding or approval, or prevention of fraud. Those are intended protocol outcomes subject to
implementation and deployment conditions outside this suite.

The JSON report's legacy `conformant_for_suite_scope` field means only that every executed test case
passed; it is not a conformance certification. JSON and JUnit reports retain the test scope and result
limitations. A passing CI job has the same limited meaning as a local run.

## Execution limits

The runner executes the adapter as a local subprocess with the invoking user's permissions. It does
not sandbox the adapter. Its timeout applies per request, and its 1 MiB response check runs after
subprocess output has been collected; neither is a general memory, output, or process-tree limit.
Use only reviewed adapters or provide isolation and resource controls in the surrounding environment.
Each request starts a fresh process, so the suite does not test persistent-session behavior or stateful
replay handling.
