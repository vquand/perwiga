import importlib.util
import json
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "fetch-npcs.py"
SPEC = importlib.util.spec_from_file_location("fetch_npcs", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class NpcSnapshotTests(unittest.TestCase):
    def test_parser_preserves_multilingual_names_and_many_locations(self):
        page = """{{Character Infobox
|type = Event NPC
|region = Liyue
|location = Mondstadt
|location2 = Liyue
|zhs = 立本
|zht = 立本
|vi = Liben
}}\n{{Description|A traveling merchant.}}\nLiben appears in {{Quest|Marvelous Merchandise}} and {{Event|Marvelous Merchandise}}."""
        record = MODULE.parse_page("Liben", 14969, page, [
            "Category:NPCs Located in Liyue",
            "Category:Event-Exclusive NPCs",
            "Category:Marvelous Merchandise",
        ])
        self.assertEqual(record["official_simplified_chinese_name"], "立本")
        self.assertEqual(record["regions"], ["Liyue"])
        self.assertEqual(record["locations"], ["Liyue", "Mondstadt"])
        self.assertEqual({item["kind"] for item in record["relationships"]}, {"event", "quest"})
        self.assertTrue(all(item["locations"] == ["Liyue", "Mondstadt"] for item in record["relationships"]))

    def test_snapshot_schema_is_namespaced_and_relationships_are_explicit(self):
        self.assertEqual(MODULE.SCHEMA, "perwiga.genshin-impact.npcs.v1")
        self.assertIn("source-backed", json.dumps({"notes": "source-backed relationships"}))

    def test_parser_captures_linked_event_title_when_template_is_absent(self):
        record = MODULE.parse_page(
            "Liben",
            14969,
            "{{Character Infobox\n|type = Event NPC\n|region = Liyue\n|location = Mondstadt\n}}\n"
            "Liben is an event-exclusive NPC that appears in [[Marvelous Merchandise]].",
            ["Category:Event-Exclusive NPCs"],
        )
        self.assertEqual(
            record["relationships"],
            [{
                "kind": "event",
                "title": "Marvelous Merchandise",
                "locations": ["Mondstadt"],
                "regions": ["Liyue"],
                "source": "explicit-linked-title",
            }],
        )


if __name__ == "__main__":
    unittest.main()
