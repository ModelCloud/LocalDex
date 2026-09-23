#!/usr/bin/env python3

from pathlib import Path
import os
import subprocess
import tempfile
import unittest

from test_install_localdex import make_archive


INSTALLER = Path(__file__).with_name("install.sh")


class InstallShTest(unittest.TestCase):
    def test_primary_installer_selects_localdex_and_preserves_data(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            home = root / "home"
            codex_home = root / "codex-home"
            bin_dir = root / "bin"
            home.mkdir()
            codex_home.mkdir()
            (codex_home / "config.toml").write_text('model = "gpt-6-sol"\n')
            (codex_home / "auth.json").write_text('{"auth":"kept"}\n')
            archive = make_archive(root, "0.155.1")
            result = subprocess.run(
                [str(INSTALLER)],
                capture_output=True,
                text=True,
                check=False,
                env={
                    **os.environ,
                    "HOME": str(home),
                    "CODEX_HOME": str(codex_home),
                    "LOCALDEX_INSTALL_DIR": str(bin_dir),
                    "LOCALDEX_ARCHIVE": str(archive),
                },
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue((bin_dir / "codex").is_symlink())
            self.assertEqual(
                (codex_home / "config.toml").read_text(), 'model = "gpt-6-sol"\n'
            )
            self.assertEqual(
                (codex_home / "auth.json").read_text(), '{"auth":"kept"}\n'
            )


if __name__ == "__main__":
    unittest.main()
