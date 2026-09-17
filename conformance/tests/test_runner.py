from __future__ import annotations

import shutil
import sys
import tempfile
import unittest
from pathlib import Path

CONFORMANCE = Path(__file__).resolve().parent.parent
REPOSITORY = CONFORMANCE.parent
sys.path.insert(0, str(CONFORMANCE))

import haps_conformance as runner  # noqa: E402


class RunnerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.suite_path = CONFORMANCE / "suite-v0.4.json"
        self.lock_path = CONFORMANCE / "haps-conformance-v0.4.lock.json"
        self.vector_root = REPOSITORY / "test_vectors" / "v0.4"

    def test_checked_in_lock_verifies(self) -> None:
        lock = runner.verify_lock(self.suite_path, self.lock_path, self.vector_root)
        self.assertEqual(lock["haps_version"], "0.4")
        self.assertEqual(len(lock["vectors"]), 13)

    def test_suite_tampering_is_detected_before_adapter_execution(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            changed = Path(temporary) / "suite-v0.4.json"
            shutil.copy2(self.suite_path, changed)
            changed.write_text(changed.read_text(encoding="utf-8") + "\n", encoding="utf-8")
            with self.assertRaises(runner.ConfigurationError):
                runner.verify_lock(changed, self.lock_path, self.vector_root)

    def test_naive_json_adapter_is_refuted(self) -> None:
        suite = runner.load_json(self.suite_path)
        adapter = Path(__file__).resolve().parent / "naive_adapter.py"
        _, results = runner.run_suite(
            suite,
            self.vector_root,
            [sys.executable, str(adapter)],
            timeout=5,
        )
        failed = {result.id for result in results if not result.passed}
        self.assertIn("vec-02-edge-canonicalization", failed)
        self.assertIn("vec-03-fractional-number", failed)
        self.assertIn("p3-duplicate-key", failed)

    def test_reversed_variant_preserves_json_value(self) -> None:
        source = '{"a":{"b":1,"c":2},"d":[{"e":3,"f":4}]}'
        reversed_source = runner.reversed_compact(source)
        self.assertNotEqual(source, reversed_source)
        self.assertEqual(
            __import__("json").loads(source),
            __import__("json").loads(reversed_source),
        )


if __name__ == "__main__":
    unittest.main()
