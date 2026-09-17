# Vendored test vectors

The normative source is [iProov/HAPS](https://github.com/iProov/HAPS), directory
`test_vectors/v0.4/`.

    copied from commit: 11ed170b3121177d971ea7f66c87981656d03daa

All files in `v0.4/`, including the README, are copied verbatim from this specification commit.
The lock records their exact bytes; local digest verification does not independently authenticate
upstream provenance.

Vendoring avoids fetching the specification when running vector tests. A first Rust build may still
need to download Cargo dependencies. Compare a sibling specification checkout without changing files:

```sh
diff -ru ../HAPS/test_vectors/v0.4 test_vectors/v0.4
```

For an intentional maintainer refresh, from the reference repository root:

```sh
./scripts/sync-vectors.sh ../HAPS
```

The script reports differences, **overwrites** the vendored vector directory and records the source
HEAD; it does not fail merely because it found drift. A dirty source checkout can include changes not
in that HEAD. Review the source state and copied diff, update the provenance above (including any
uncommitted source changes) and `SPEC_COMMIT` / provenance note in `conformance/update_lock.py`, then run:

```sh
python3 conformance/update_lock.py
python3 scripts/sync_plugin_conformance.py --write
python3 conformance/haps_conformance.py verify-lock
python3 scripts/sync_plugin_conformance.py --check
```

A passing local lock check detects byte drift against the lock; it does not show that upstream is
unchanged or establish protocol conformance.
