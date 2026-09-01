import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "fetch-heroes-iii.py"
SPEC = importlib.util.spec_from_file_location("fetch_heroes_iii", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class HeroesIiiCrawlerTests(unittest.TestCase):
    def test_jsonc_parser_strips_comments_without_damaging_strings(self):
        payload = MODULE.parse_jsonc(
            '''{
              // source metadata
              "url": "https://example.com/path//kept",
              "entry": {"index": 7} /* pinned structural ID */
            }'''
        )

        self.assertEqual(payload["url"], "https://example.com/path//kept")
        self.assertEqual(payload["entry"]["index"], 7)

    def test_jsonc_parser_accepts_vcmi_trailing_commas_without_damaging_strings(self):
        payload = MODULE.parse_jsonc(
            '''{
              "literal": "kept,}",
              "items": [1, 2,],
              "entry": {"index": 7,},
            }'''
        )

        self.assertEqual(payload["literal"], "kept,}")
        self.assertEqual(payload["items"], [1, 2])
        self.assertEqual(payload["entry"], {"index": 7})

    def test_template_parser_handles_nested_templates_and_named_fields(self):
        text = """{{CreatureNew
        | name = Halberdier
        | attack = {{gt|6}}
        | damage = {{gt|2}}–3
        | abilities = Immune to {{gl|jousting|Jousting}} bonus.
        }}"""

        fields = MODULE.template_fields(text, "CreatureNew")

        self.assertEqual(fields["name"], "Halberdier")
        self.assertEqual(MODULE.clean_wikitext(fields["attack"]), "6")
        self.assertEqual(MODULE.clean_wikitext(fields["damage"]), "2–3")
        self.assertEqual(
            MODULE.clean_wikitext(fields["abilities"]),
            "Immune to Jousting bonus.",
        )

    def test_artifact_rows_keep_complete_editions_and_exclude_hota(self):
        text = """
        {{Ar|Centaur's Axe|roe|Weapon|Treasure|2000|+2 {{Ps|Attack}}||}}
        {{Ar|Armageddon's Blade|ab|Weapon|Relic|50000|Expert {{Sn|Armageddon}}||}}
        {{Ar|Titan's Thunder|sod|Combo|Relic|40000|Expert {{Sn|Titan's Lightning Bolt}}||}}
        {{Ar|Trident of Dominion|hota|Weapon|Major|7000|+7 {{Ps|Attack}}|||only=hota}}
        """

        rows = MODULE.parse_artifact_rows(text)

        self.assertEqual(
            [row["name"] for row in rows],
            ["Centaur's Axe", "Armageddon's Blade", "Titan's Thunder"],
        )
        self.assertEqual([row["edition"] for row in rows], ["roe", "ab", "sod"])

    def test_snapshot_write_refuses_an_unreviewed_catalog_shrink(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "heroes-iii.json"
            output.write_text(
                json.dumps({"catalog_scope": {"included_records": 10}}),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(ValueError, "refusing to shrink"):
                MODULE.write_snapshot(
                    {"catalog_scope": {"included_records": 9}},
                    output,
                    allow_shrink=False,
                )

    def test_expected_complete_catalog_counts_are_explicit(self):
        self.assertEqual(
            MODULE.EXPECTED_COUNTS,
            {
                "town": 9,
                "hero": 156,
                "creature": 141,
                "artifact": 141,
                "spell": 70,
            },
        )

    def test_catalog_validation_rejects_duplicate_source_keys(self):
        records = [
            {"source_key": "homm3-complete-hero-0", "entity_type": "hero"},
            {"source_key": "homm3-complete-hero-0", "entity_type": "hero"},
        ]

        with self.assertRaisesRegex(ValueError, "duplicate source key"):
            MODULE.validate_catalog_records(records, {"hero": 2})

    def test_catalog_validation_requires_exact_type_counts(self):
        records = [
            {"source_key": "homm3-complete-town-0", "entity_type": "town"},
            {"source_key": "homm3-complete-hero-0", "entity_type": "hero"},
        ]

        with self.assertRaisesRegex(ValueError, "hero catalog has 1 records; expected 2"):
            MODULE.validate_catalog_records(records, {"town": 1, "hero": 2})

    def test_camel_case_source_keys_become_reviewable_titles(self):
        self.assertEqual(MODULE.title_from_key("lordHaart"), "Lord Haart")
        self.assertEqual(MODULE.title_from_key("blackshardOfTheDeadKnight"), "Blackshard Of The Dead Knight")

    def test_wiki_page_helpers_preserve_revision_provenance_and_selected_facts(self):
        page = {
            "pageid": 42,
            "title": "Magic Arrow",
            "revisions": [
                {
                    "revid": 73,
                    "timestamp": "2026-08-31T12:00:00Z",
                    "slots": {
                        "main": {
                            "*": "{{Spell| school = {{Sm|Air Magic}} | level = 1 | cost = 5 }}"
                        }
                    },
                }
            ],
        }

        source = MODULE.wiki_page_source(page)
        facts = MODULE.facts_from_template(
            MODULE.wiki_page_content(page),
            "Spell",
            (("School", "school"), ("Level", "level"), ("Cost", "cost")),
        )

        self.assertEqual(source["page_id"], 42)
        self.assertEqual(source["revision_id"], 73)
        self.assertEqual(source["revision_timestamp"], "2026-08-31T12:00:00Z")
        self.assertEqual(
            facts,
            [
                {"label": "School", "value": "Air Magic"},
                {"label": "Level", "value": "1"},
                {"label": "Cost", "value": "5"},
            ],
        )


if __name__ == "__main__":
    unittest.main()
