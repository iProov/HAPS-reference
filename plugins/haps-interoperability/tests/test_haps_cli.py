from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path


PLUGIN_ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = PLUGIN_ROOT.parents[1]
CLI = PLUGIN_ROOT / "bin" / "haps"
LEGACY_INSTALLER = PLUGIN_ROOT / "scripts" / "install_conformance_kit.py"
SOURCE = PLUGIN_ROOT / "assets" / "conformance"


def portable_files(root: Path) -> set[Path]:
    return {
        path.relative_to(root)
        for path in root.rglob("*")
        if path.is_file()
        and "__pycache__" not in path.parts
        and path.suffix not in {".pyc", ".pyo"}
    }


class HapsCliTests(unittest.TestCase):
    def run_cli(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(CLI), *arguments],
            text=True,
            capture_output=True,
            check=False,
        )

    def test_init_installs_only_the_portable_kit_and_is_idempotent(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary)
            first = self.run_cli("init", str(target))
            self.assertEqual(first.returncode, 0, first.stderr)
            self.assertIn("No language implementation was generated.", first.stdout)

            installed = target / ".haps" / "conformance"
            self.assertEqual(portable_files(installed), portable_files(SOURCE))
            self.assertEqual({path.name for path in target.iterdir()}, {".haps"})
            self.assertFalse(any("__pycache__" in path.parts for path in installed.rglob("*")))
            for name in ("LICENSE", "NOTICE"):
                expected = (REPOSITORY / name).read_bytes()
                for copy in (PLUGIN_ROOT / name, SOURCE / name, installed / name):
                    with self.subTest(licensing_copy=str(copy)):
                        self.assertEqual(copy.read_bytes(), expected)

            verify = subprocess.run(
                ["python3", str(installed / "haps_conformance.py"), "verify-lock"],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(verify.returncode, 0, verify.stderr)

            second = self.run_cli("init", str(target))
            self.assertEqual(second.returncode, 0, second.stderr)
            self.assertIn("already current", second.stdout)

    def test_drift_fails_closed_until_explicit_update(self) -> None:
        for name in ("suite-v0.4.json", "LICENSE", "NOTICE"):
            with self.subTest(changed_file=name), tempfile.TemporaryDirectory() as temporary:
                target = Path(temporary)
                self.assertEqual(self.run_cli("init", str(target)).returncode, 0)
                changed = target / ".haps" / "conformance" / name
                changed.write_text(changed.read_text(encoding="utf-8") + "\n", encoding="utf-8")

                check = self.run_cli("check", str(target))
                self.assertEqual(check.returncode, 1)
                refused = self.run_cli("init", str(target))
                self.assertEqual(refused.returncode, 2)

                updated = self.run_cli("init", str(target), "--update")
                self.assertEqual(updated.returncode, 0, updated.stderr)
                self.assertEqual(self.run_cli("check", str(target)).returncode, 0)

    def test_non_directory_target_is_reported_without_a_traceback(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary) / "not-a-directory"
            target.write_text("occupied", encoding="utf-8")
            result = self.run_cli("init", str(target))
            self.assertEqual(result.returncode, 2)
            self.assertIn("target is not a directory", result.stderr)
            self.assertNotIn("Traceback", result.stderr)

    def test_legacy_installer_remains_compatible(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary)
            install = subprocess.run(
                ["python3", str(LEGACY_INSTALLER), "--target", str(target)],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(install.returncode, 0, install.stderr)
            check = subprocess.run(
                [
                    "python3",
                    str(LEGACY_INSTALLER),
                    "--target",
                    str(target),
                    "--check",
                ],
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(check.returncode, 0, check.stderr)


if __name__ == "__main__":
    unittest.main()
