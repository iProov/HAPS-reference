# CI integration

This kit supports an experimental draft specification and partial reference implementation; it is
not production-ready. A passing CI job records the finite suite results, not full protocol conformance
or evidence that a person saw and approved the correct action.

Use the project's existing CI provider and pinned language toolchain. The CI job must execute the same
adapter binary and kit used locally; do not substitute agent evaluation for the deterministic runner.

The essential sequence is:

```sh
python3 .haps/conformance/haps_conformance.py verify-lock
<project-native adapter build command>
python3 .haps/conformance/haps_conformance.py test \
  --adapter <built-adapter> \
  --json-report build/haps-conformance.json \
  --junit-report build/haps-conformance.xml
```

Upload both reports even when the suite fails. If the CI platform requires it, preserve the runner exit
code while arranging artifact upload in an `always()`/post-job step.

## GitHub Actions shape

Integrate this shape into an existing workflow and replace the build command and adapter path with real,
repository-specific values:

```yaml
- name: Verify pinned HAPS kit
  run: python3 .haps/conformance/haps_conformance.py verify-lock

- name: Build HAPS adapter
  run: <native build command>

- name: Run HAPS interoperability suite
  run: |
    python3 .haps/conformance/haps_conformance.py test \
      --adapter <built-adapter> \
      --json-report build/haps-conformance.json \
      --junit-report build/haps-conformance.xml

- name: Upload HAPS reports
  if: always()
  uses: actions/upload-artifact@v7
  with:
    name: haps-conformance
    path: |
      build/haps-conformance.json
      build/haps-conformance.xml
```

Keep dependency caches scoped to the project's lockfiles. The HAPS runner and vectors are vendored and
digest-checked, so they do not require network access. Adapter builds or adapter code may have separate
network requirements. The runner executes the adapter without sandboxing; use the CI environment's
isolation and resource controls as needed.

## Review checks

- The CI trigger covers the repository's actual default/release branches.
- Lock verification occurs before the adapter executes.
- The native build is non-interactive and uses checked-in dependency locks.
- The adapter command does not use a shell string assembled from untrusted input.
- A failing suite fails the job.
- Report wording retains the finite canonicalization/hash scope and experimental status; it does not
  claim full conformance, formal proof, correct display, human approval, or production readiness.
