---
name: haps-conformance
description: "Test an existing HAPS canonicalization/hash implementation with the pinned language-neutral suite and add the same gate to CI. Use for HAPS conformance checks, adapter debugging, CI integration, local vector digest checks, or explicit HAPS kit upgrades. This experimental kit is not production-ready and does not establish full protocol conformance, correct display, or human approval."
---

# Test HAPS interoperability

HAPS is an experimental draft specification with a partial reference implementation. This kit is not
production-ready. Passing the supplied tests does not establish full protocol conformance or that a
person saw and approved the correct action.

Use the checked-in runner and lock to record results for the listed cases. The agent may diagnose and
repair an adapter, but it must not reinterpret a mismatch as an acceptable variant.

## Workflow

1. Inspect repository instructions, status, language/build tooling, current tests, and CI configuration.
2. If `.haps/conformance` is absent, resolve this skill's plugin root and run
   `<plugin-root>/bin/haps init <repo-root>`. It installs the locked test materials without generating
   or selecting a language implementation.
3. Read `.haps/conformance/adapter-protocol-v1.md` and identify or build the adapter executable.
4. Run `python3 .haps/conformance/haps_conformance.py verify-lock` before executing target code. Stop on
   any lock or vector digest mismatch.
5. Build the adapter using the project's normal pinned toolchain, then run the suite with JSON and JUnit
   reports.
6. Diagnose failures from the protocol response and discriminating vector. Change implementation code,
   not expected hashes or the lock.
7. Read [references/ci-integration.md](references/ci-integration.md) and add the same build, lock check,
   and suite command to the repository's existing CI provider.

Treat `haps-conformance-v0.4.lock.json`, `suite-v0.4.json`, the protocol, and vendored vectors as one
versioned unit. Never run the lock-regeneration script merely to make a test pass. Update that unit only
for an intentional, reviewed kit/specification update, retain accurate source provenance, inspect
vector drift, and rerun the relevant native and interoperability tests. `verify-lock` compares local
bytes with the lock; it does not authenticate the recorded upstream commit.

## Evidence boundary

Always state:

- the HAPS suite identifier and pinned specification commit;
- adapter implementation and version;
- passed and failed case counts;
- whether the result was local or CI;
- that the scope is finite canonicalization, hashing, and malformed-input examples only;
- that this is experimental and not production-ready, and passing does not establish full protocol
  conformance or that a person saw and approved the correct action.

Do not infer signature verification, issuer trust, audience/freshness enforcement, replay safety,
provider certification, identity binding, presence-signal quality, a PoHP level, correct display, human
understanding/approval, or full HAPS PP/RP conformance from this suite. Do not describe a passing result
as formal verification or proof that an implementation cannot fail or prevents all fraud. Security and
approval are intended protocol outcomes subject to implementation and deployment conditions outside
these tests. The report's legacy `conformant_for_suite_scope` field means only that the executed cases
passed.

The runner executes local adapter processes without sandboxing. Its output-size check happens after
capture, and its timeout applies per request. Review the adapter or use environment-level isolation
and resource limits; the kit does not assess resource exhaustion or persistent-session behavior.
