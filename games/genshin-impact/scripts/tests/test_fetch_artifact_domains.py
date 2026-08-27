from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "fetch-artifact-domains.py"
SPEC = importlib.util.spec_from_file_location("fetch_artifact_domains", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
domains = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(domains)


class ArtifactDomainSnapshotTests(unittest.TestCase):
    def challenge(self, name: str, entrance: str, set_name: str, level: int) -> dict[str, object]:
        return {
            "id": 5200 + level,
            "name": name,
            "regionId": 3,
            "regionName": "Inazuma",
            "entranceId": 361,
            "entranceName": entrance,
            "domainType": "UI_ABYSSUS_RELIC",
            "domainText": "Artifacts",
            "description": "An artifact domain.",
            "recommendedLevel": level,
            "recommendedElements": ["Pyro", "Cryo"],
            "unlockRank": 30,
            "rewardPreview": [{"id": 400178, "name": set_name, "rarity": 4}],
        }

    def write_fixture(self, root: Path) -> None:
        import json

        localized = {
            "English": ("Momiji-Dyed Court", "Shimenawa's Reminiscence"),
            "ChineseSimplified": ("椛染之庭", "追忆之注连"),
            "ChineseTraditional": ("椛染之庭", "追憶之注連"),
            "Vietnamese": ("Sân Vườn Momiji", "Dòng Hồi Ức Bất Tận"),
        }
        for locale, (entrance, set_name) in localized.items():
            directory = root / "src" / "data" / locale / "domains"
            directory.mkdir(parents=True, exist_ok=True)
            for level in (59, 69):
                payload = self.challenge(
                    f"Domain of Blessing: Autumn Hunt {level}", entrance, set_name, level
                )
                (directory / f"autumnhunt{level}.json").write_text(
                    json.dumps(payload), encoding="utf-8"
                )

    def test_difficulty_stages_group_into_one_domain_entrance(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.write_fixture(root)
            snapshot = domains.build_snapshot(
                root,
                artifact_set_names={"Shimenawa's Reminiscence": "genshin-data-artifact-set-15018"},
                checked_at="2026-08-26",
                source_commit="b" * 40,
            )

        self.assertEqual(snapshot["catalog_scope"]["domain_entrances"], 1)
        self.assertEqual(snapshot["catalog_scope"]["challenge_stages"], 2)
        domain = snapshot["domains"][0]
        self.assertEqual(domain["source_key"], "genshin-data-artifact-domain-361")
        self.assertEqual(domain["official_simplified_chinese_name"], "椛染之庭")
        self.assertEqual(domain["region"], "Inazuma")
        self.assertEqual(domain["challenge_stage_ids"], [5259, 5269])
        self.assertEqual(
            domain["hosted_artifact_set_source_keys"],
            ["genshin-data-artifact-set-15018"],
        )

    def test_non_artifact_domains_are_excluded(self) -> None:
        challenge = self.challenge("Test", "Test Entrance", "Test Set", 59)
        challenge["domainType"] = "UI_ABYSSUS_AVATAR_TALENT"
        self.assertFalse(domains.is_artifact_challenge(challenge))


if __name__ == "__main__":
    unittest.main()
