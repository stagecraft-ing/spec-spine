"""Spec: specs/008-python-distribution/spec.md.

Network- and disk-free unit test of the (os, cpu) -> target mapping, the wheel
platform tags, and the unsupported-host refusal -- the Python parallel of
npm/test/platform.test.js. Stdlib unittest so it runs with no third-party deps:
`python -m unittest discover -s test`.
"""

import unittest

from spec_spine import platform_map as pm


class PlatformMapTest(unittest.TestCase):
    def test_maps_every_supported_target(self):
        cases = [
            ("darwin", "arm64", "aarch64-apple-darwin", "macosx_11_0_arm64", "spec-spine"),
            ("darwin", "x64", "x86_64-apple-darwin", "macosx_10_12_x86_64", "spec-spine"),
            ("linux", "x64", "x86_64-unknown-linux-gnu", "manylinux_2_17_x86_64", "spec-spine"),
            ("linux", "arm64", "aarch64-unknown-linux-gnu", "manylinux_2_17_aarch64", "spec-spine"),
            ("win32", "x64", "x86_64-pc-windows-msvc", "win_amd64", "spec-spine.exe"),
        ]
        for os_name, cpu, triple, wheel_plat, binname in cases:
            t = pm.target_for(os_name, cpu)
            self.assertEqual(t["triple"], triple, f"{os_name}-{cpu} triple")
            self.assertEqual(t["wheel_platform"], wheel_plat, f"{os_name}-{cpu} wheel tag")
            self.assertEqual(t["binary_name"], binname, f"{os_name}-{cpu} binary")
            self.assertEqual(t["key"], f"{os_name}-{cpu}")

    def test_table_is_exactly_the_five_release_triples(self):
        self.assertEqual(
            sorted(pm.TARGETS),
            ["darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64", "win32-x64"],
        )

    def test_triples_match_release_yml_matrix(self):
        # These five strings must equal release.yml's build matrix exactly.
        self.assertEqual(
            sorted(rec["triple"] for rec in pm.TARGETS.values()),
            [
                "aarch64-apple-darwin",
                "aarch64-unknown-linux-gnu",
                "x86_64-apple-darwin",
                "x86_64-pc-windows-msvc",
                "x86_64-unknown-linux-gnu",
            ],
        )

    def test_refuses_unsupported_targets_with_source_hint(self):
        for os_name, cpu in [("win32", "arm64"), ("linux", "ia32"),
                             ("freebsd", "x64"), ("darwin", "ppc64")]:
            with self.assertRaises(pm.UnsupportedHostError) as ctx:
                pm.target_for(os_name, cpu)
            msg = str(ctx.exception)
            self.assertIn(f"{os_name}-{cpu}", msg)
            self.assertIn("cargo install spec-spine-cli", msg)

    def test_musl_message_mentions_glibc_image(self):
        msg = pm.unsupported_message("linux", "x64", reason="musl")
        self.assertIn("musl", msg)
        self.assertIn("glibc-based image", msg)
        self.assertIn("cargo install spec-spine-cli", msg)

    def test_binary_name(self):
        self.assertEqual(pm.binary_name(False), "spec-spine")
        self.assertEqual(pm.binary_name(True), "spec-spine.exe")


if __name__ == "__main__":
    unittest.main()
