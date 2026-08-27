from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "fetch-artifact-thumbnails.py"
SPEC = importlib.util.spec_from_file_location("fetch_artifact_thumbnails", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
thumbnails = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(thumbnails)


class ArtifactThumbnailTests(unittest.TestCase):
    def record(self, source_key: str, url: str) -> dict[str, str]:
        return {"source_key": source_key, "thumbnail_asset": f"{source_key}.png", "thumbnail_source_url": url}

    def test_snapshot_accepts_distinct_set_and_piece_assets(self) -> None:
        snapshot = {
            "schema": thumbnails.SCHEMA,
            "catalog_scope": {"artifact_sets": 1, "artifact_pieces": 1},
            "artifact_sets": [self.record("genshin-data-artifact-set-1", "https://act-webstatic.hoyoverse.com/set.png")],
            "artifact_pieces": [self.record("genshin-data-artifact-piece-1-flower", "https://upload-os-bbs.mihoyo.com/piece.png")],
        }
        self.assertEqual(len(thumbnails.validate_snapshot(snapshot)), 2)

    def test_rejects_path_traversal_and_foreign_hosts(self) -> None:
        with self.assertRaisesRegex(ValueError, "unsafe"):
            thumbnails.validated_asset(self.record("genshin-data-artifact-piece-../escape", "https://upload-os-bbs.mihoyo.com/piece.png"))
        with self.assertRaisesRegex(ValueError, "untrusted"):
            thumbnails.validated_asset(self.record("genshin-data-artifact-set-1", "https://example.com/set.png"))


if __name__ == "__main__":
    unittest.main()
