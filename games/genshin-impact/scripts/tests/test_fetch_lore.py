import importlib.util
import unittest


SCRIPT_PATH = __import__("pathlib").Path(__file__).parents[1] / "fetch-lore.py"
SPEC = importlib.util.spec_from_file_location("fetch_lore", SCRIPT_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Unable to load {SCRIPT_PATH}")
fetch_lore = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fetch_lore)


BOOK_PAGE = """{{Book Collection Infobox
|quality   = 4
|region_lore = Unknown
|region_location = Mondstadt
|volumes   = 7
|vol1      = Automatically obtained during {{Quest|Lost Book}}
}}
'''The Pale Princess and the Six Pygmies''' is a [[Book Collection]].

The [[Abyss Order]] attempts to steal Vol. 1 during {{Quest|Lost Book}}.

==Other Languages==
{{Other Languages
|en = The Pale Princess and the Six Pygmies
|zhs = 白之公主与六侏儒
|zht = 白之公主與六侏儒
|vi = Công Chúa Trắng và 6 Chú Lùn
}}
"""

QUEST_PAGE = """{{Quest Infobox
|id            = 10101
|type          = Story
|chapter       = Tempus Fugit Chapter
|actNum        = I
|part          = 2
|character     = Lisa
|prev          = Troublesome Work (Quest)
|rewards       = Adventure EXP*425;1000 Years of Loneliness*1;The Pale Princess and the Six Pygmies
|characters    = Lisa;Paimon;Traveler
}}
'''''Lost Book''''' is the second and final part of {{Quest|Troublesome Work}}.
"""


class LoreCrawlerTests(unittest.TestCase):
    def test_book_parser_preserves_metadata_and_explicit_quest_links(self):
        record = fetch_lore.parse_book_page(
            "The Pale Princess and the Six Pygmies",
            1234,
            BOOK_PAGE,
            ["Category:4-Star Book Collections", "Category:Released in Version 1.0"],
        )

        self.assertEqual(record["source_key"], "genshin-fandom-book-1234")
        self.assertEqual(record["book_kind"], "collection")
        self.assertEqual(record["official_simplified_chinese_name"], "白之公主与六侏儒")
        self.assertEqual(record["official_vietnamese_name"], "Công Chúa Trắng và 6 Chú Lùn")
        self.assertEqual(record["release_version"], "1.0")
        self.assertEqual(record["quest_links"], ["Lost Book"])

    def test_books_category_index_page_is_not_a_book_record(self):
        record = fetch_lore.parse_book_page(
            "Book",
            2569,
            "{{Terminology Infobox\n|type = Items\n}}",
            ["Category:Books"],
        )

        self.assertEqual(record["book_kind"], "index")

    def test_quest_parser_connects_characters_books_previous_quest_and_release(self):
        record = fetch_lore.parse_quest_page(
            "Lost Book",
            5678,
            QUEST_PAGE,
            ["Category:Quest", "Category:Lisa Appearances", "Category:Released in Version 1.0"],
            book_titles={"The Pale Princess and the Six Pygmies", "1000 Years of Loneliness"},
            character_names={"Lisa"},
            npc_names={"Paimon"},
        )

        self.assertEqual(record["source_key"], "genshin-fandom-quest-5678")
        self.assertEqual(record["record_kind"], "quest")
        self.assertEqual(record["release_version"], "1.0")
        self.assertEqual(record["previous_quest_title"], "Troublesome Work (Quest)")
        self.assertEqual(
            {(item["entity_type"], item["name"]) for item in record["entity_links"]},
            {("character", "Lisa"), ("npc", "Paimon"), ("unknown", "Traveler")},
        )
        self.assertEqual(
            {item["title"] for item in record["book_links"]},
            {"1000 Years of Loneliness", "The Pale Princess and the Six Pygmies"},
        )

    def test_quest_parser_keeps_adjacent_next_and_rewards_fields_separate(self):
        record = fetch_lore.parse_quest_page(
            "Festival Quest",
            9012,
            "{{Quest Infobox\n|next = Festival Afterword|rewards = Mora*20,000\n}}",
            ["Category:Quest"],
            book_titles=set(),
            character_names=set(),
            npc_names=set(),
        )

        self.assertEqual(record["next_quest_title"], "Festival Afterword")
        self.assertEqual(record["unresolved_quest_links"], [])

    def test_unknown_character_is_retained_as_provisional_link(self):
        record = fetch_lore.parse_quest_page(
            "A Quest",
            3456,
            "{{Quest Infobox\n|characters = Traveler\n}}",
            ["Category:Quest"],
            book_titles=set(),
            character_names=set(),
            npc_names=set(),
        )

        self.assertEqual(record["entity_links"], [{
            "entity_type": "unknown",
            "name": "Traveler",
            "source_key": None,
            "source": "infobox",
        }])

    def test_quest_like_infobox_is_retained_for_expanded_links(self):
        record = fetch_lore.parse_quest_page(
            "Sunchildren Hide and Seek",
            7890,
            "{{Hidden Exploration Objectives Infobox\n|requirement = Complete the objective\n}}",
            ["Category:Quest-Like"],
            book_titles=set(),
            character_names=set(),
            npc_names=set(),
        )

        self.assertEqual(record["record_kind"], "quest_like")

    def test_redirect_alias_resolves_to_existing_quest_record(self):
        record = {
            "source_key": "quest-1",
            "page_title": "Knight and Deaconess, Ready for Battle!",
            "official_english_name": "Knight and Deaconess, Ready for Battle!",
            "redirect_titles": ["Home-Made Chilibrew"],
        }

        self.assertIs(fetch_lore.resolve_title("Home-Made Chilibrew", fetch_lore.index_by_title([record])), record)

    def test_exact_title_wins_when_quest_names_are_ambiguous(self):
        regular = {
            "source_key": "quest-1",
            "page_title": "Behind the Scenes",
            "official_english_name": "Behind the Scenes",
        }
        event = {
            "source_key": "quest-2",
            "page_title": "Behind the Scenes (Event Quest)",
            "official_english_name": "Behind the Scenes",
        }

        index = fetch_lore.index_by_title([regular, event])
        self.assertIs(fetch_lore.resolve_title("Behind the Scenes", index), regular)
        self.assertIs(fetch_lore.resolve_title("Behind the Scenes (Event Quest)", index), event)

    def test_case_sensitive_title_wins_when_wiki_titles_differ_only_by_case(self):
        upper = {
            "source_key": "quest-1",
            "page_title": "Everlasting As the Moon",
            "official_english_name": "Everlasting As the Moon",
        }
        lower = {
            "source_key": "quest-2",
            "page_title": "Everlasting as the Moon",
            "official_english_name": "Everlasting as the Moon",
        }

        index = fetch_lore.index_by_title([upper, lower])
        self.assertIs(fetch_lore.resolve_title("Everlasting As the Moon", index), upper)
        self.assertIs(fetch_lore.resolve_title("Everlasting as the Moon", index), lower)

    def test_join_validation_rejects_missing_cross_catalog_targets(self):
        books = [{"source_key": "book-1", "official_english_name": "Known Book"}]
        quests = [{
            "source_key": "quest-1",
            "book_links": [{"title": "Missing Book", "source_key": None}],
            "previous_quest_source_key": "missing-quest",
        }]

        with self.assertRaisesRegex(ValueError, "Missing Book|missing-quest"):
            fetch_lore.validate_cross_catalog_links(books, quests)

    def test_lore_batch_has_one_explicit_event_path_for_each_quest(self):
        books = [{
            "source_key": "book-1",
            "page_id": 1234,
            "page_title": "Known Book",
            "official_english_name": "Known Book",
            "page_url": "https://genshin-impact.fandom.com/wiki/Known_Book",
            "release_version": "1.0",
        }]
        quests = [{
            "source_key": "quest-1",
            "page_id": 5678,
            "page_title": "Known Quest",
            "official_english_name": "Known Quest",
            "page_url": "https://genshin-impact.fandom.com/wiki/Known_Quest",
            "release_version": "1.0",
            "record_kind": "quest",
            "entity_links": [{"entity_type": "character", "name": "Lisa", "source_key": "char-1"}],
            "book_links": [{"title": "Known Book", "source_key": "book-1", "source": "reward"}],
            "previous_quest_source_key": None,
        }]

        batch = fetch_lore.build_lore_batch(
            books,
            quests,
            characters=[{"source_key": "char-1", "official_english_name": "Lisa"}],
            npcs=[],
            checked_at="2026-09-02",
        )

        self.assertEqual(len(batch["events"]), 1)
        self.assertEqual(batch["events"][0]["time"]["start_period_key"], "release-version-1-0")
        self.assertIn("book-book-1", [item["subject_key"] for item in batch["events"][0]["involvements"]])
        self.assertIn("character-char-1", [item["subject_key"] for item in batch["events"][0]["involvements"]])


if __name__ == "__main__":
    unittest.main()
