# Arknights: Endfield Multilingual Naming Policy

This is an English adaptation of the user-authored *Arknights Endfield Multilingual Naming Skill*. It consolidates repeated material while preserving the source document's unique rules and workflows. The original document is source material, not an authority for current game facts. Reverify every factual example before using it in Perwiga.

## Purpose

Build a reliable multilingual naming index for **Arknights: Endfield / 明日方舟：终末地**, especially for:

- Researching Chinese fanfiction and web novels
- Resolving names found in Chinese sources
- Maintaining a Vietnamese-facing glossary
- Populating Perwiga wiki entities and aliases
- Recovering names from Hán-Việt converters or imperfect machine translation

The index should distinguish four naming layers:

1. Official English used by the game or publisher
2. Official Simplified Chinese used by the game or publisher
3. Chinese community, fandom, and web-novel aliases or slang
4. Vietnamese, with official localization preferred and all generated fallbacks clearly labeled

Perwiga stores official Vietnamese separately from generated Vietnamese. Hán-Việt and converter renderings belong in aliases or other information, not in the official Vietnamese field.

## Entity Coverage

Use the policy for all Endfield entity types, including:

- Characters, operators, NPCs, nicknames, titles, and codenames
- Places, regions, settlements, facilities, landmarks, and anomalous zones
- Factions, organizations, squads, teams, and institutions
- Weapons, equipment, devices, and technology
- Skills, talents, ultimates, combat mechanics, statuses, and effects
- Items, materials, resources, products, food, and quest objects
- Lore concepts, phenomena, substances, civilizations, and historical terms
- Enemy families, standard enemies, elites, bosses, and hostile devices
- Wildlife, plants, fungi, ecology-related resources, and environmental hazards
- Archives, quests, missions, and other named terms

Do not force every game-specific entity into one universal taxonomy. Preserve the Endfield module's distinctions and map them to shared Perwiga concepts only where the implemented schema supports that mapping.

## Source Hierarchy

Prefer sources in this order:

1. Official Hypergryph Endfield sources, including `endfield.hypergryph.com`
2. Official international Endfield sources, including `endfield.games`
3. In-game localization, official announcements, trailers, social accounts, and published documents
4. Well-sourced wikis or databases that identify their evidence
5. Chinese community platforms such as Bilibili, NGA, Tieba, Weibo, and Baidu Baike
6. Chinese web-novel platforms such as Qidian, Faloo, Ciweimao, Fanqie, and similar sites
7. Fan translations, machine conversions, and reasoned inference

Search results and snippets are discovery aids, not final evidence. Open the underlying page where possible. A lower-ranked source cannot turn fan terminology into an official name.

## Core Naming Rules

### Official text takes priority

- Keep the current official English and Chinese forms as primary names.
- Verify each language independently; do not derive one official field from another.
- If an official field is unknown, leave it empty rather than guessing.
- Preserve punctuation, capitalization, spacing, script, accents, diacritics, and mixed-language spellings.

### Community aliases must be attested

- Never invent a Chinese nickname or web-novel alias.
- If no stable alias is found, write `—` or `No common alias found` in a research response.
- Record rare or source-specific usage with a lower confidence grade.
- Label tone and context when known: `[meme]`, `[fanfic]`, `[nickname]`, `[beta name]`, `[affectionate]`, or `[mocking]`.
- Do not mistake a translated official name, shortened search phrase, typo, or one author's invention for broad community usage.

### Vietnamese provenance must remain explicit

Use this priority:

1. Official Vietnamese localization
2. A generated Vietnamese translation stored separately from official text
3. Hán-Việt for clear Chinese personal or place names, labeled `[Hán-Việt]`
4. Converter output, labeled `[Convert]`
5. English retention or a cautious transliteration for fantasy or foreign-origin names

Never present Hán-Việt, converter output, fan translation, or machine translation as official Vietnamese. If multiple generated variants are useful for search, retain them as aliases with provenance instead of selecting one silently.

### Current and legacy names are separate

- Use the current official name as primary.
- Retain beta, test, obsolete, and previous official forms as aliases.
- Label the period or version when known.
- Do not merge two distinct entities merely because one name changed or resembles another.

### Project-preferred names do not replace official forms

The user may choose a stable preferred rendering for prose or display. Store it as a project-preferred value when supported. Continue to retain official English, official Chinese, official Vietnamese, and aliases for lookup and provenance.

## Research Workflow

### 1. Identify the entity

Determine:

- Entity type
- Likely game region, faction, or system
- Whether the input is current, beta, fan-created, or converter-derived
- Which language the input appears to use

### 2. Find official Chinese

Useful query patterns include:

```text
site:endfield.hypergryph.com <English name>
site:endfield.hypergryph.com <Chinese candidate>
明日方舟 终末地 <name>
终末地 <entity type> <name>
```

Confirm the exact characters and whether the result is an official proper name rather than a descriptive phrase.

### 3. Find official English

Search official international pages, in-game localization, announcements, trailers, and official media. Do not assume an English wiki title is official without evidence.

### 4. Find Chinese community and web-novel forms

Useful query patterns include:

```text
<official Chinese> 终末地
<official Chinese> 外号
<official Chinese> 别称
<official Chinese> 昵称
<official Chinese> 梗
<official Chinese> 同人
<official Chinese> 小说
<official Chinese> 起点
<official Chinese> 刺猬猫
<official Chinese> 番茄
```

Record where each alias is used, its tone, and how broadly it appears. Prefer multiple independent attestations for a claim of common usage.

### 5. Resolve Vietnamese

- Search for official Vietnamese localization first.
- If absent, keep the official field empty.
- For a generated translation, preserve the method and source text.
- For Hán-Việt or converter forms, preserve the exact variant as a labeled alias.
- Optimize aliases for recognition and reverse lookup, not literary elegance.

### 6. Cross-check identity

Verify that official Chinese, official English, Vietnamese, and aliases refer to the same entity. Check entity type, visual or narrative context, faction, region, and version where useful.

## Confidence Scale

Assign confidence to each name or assertion:

- **A — Official:** Directly confirmed by the game or publisher.
- **B — Strongly supported:** Confirmed by a well-sourced reference or several reliable sources.
- **C — Common fandom usage:** Repeated, recognizable community usage without official status.
- **D — Isolated usage:** Found in one or very few sources, such as a single novel or post.
- **E — Inference:** A reasoned candidate, transliteration, machine conversion, or unresolved reconstruction.

Do not upgrade confidence simply because a form sounds plausible.

## Output Formats

### Single entity

```text
Type:
Official English:
Official Simplified Chinese:
Pinyin:
Official Vietnamese:
Generated Vietnamese:
Aliases and other information:
Current/beta/legacy status:
Project-preferred name:
Sources:
Confidence by assertion:
Notes and unresolved questions:
```

### Batch glossary

Use columns equivalent to:

| Type | Official English | Official Chinese | Chinese community/novel forms | Official Vietnamese | Generated Vietnamese | Other aliases | Notes and confidence |
|---|---|---|---|---|---|---|---|

For a default glossary, organize sections in this order:

1. Characters
2. Places
3. Factions and organizations
4. Weapons and equipment
5. Skills and combat mechanics
6. Items and materials
7. Lore concepts and phenomena
8. Enemies and enemy families
9. Ecology: wildlife, plants, and fungi
10. Archives, quests, and other named terms

## Fanfiction and Web-Novel Search Support

When the goal is finding works rather than merely naming an entity, return:

- The exact official Simplified Chinese name
- Attested Chinese aliases and abbreviations
- Labeled fanfiction or web-novel terms
- Search-ready query combinations
- Platform-specific suggestions only when current evidence supports them

Example query construction:

```text
明日方舟 终末地 <official Chinese>
终末地 <alias> 同人
终末地 <character or faction> 小说
```

Do not claim that a work exists unless it was actually found.

## Reverse Lookup from Converter Text

Use this workflow for broken Hán-Việt, machine-converted, OCR-derived, or copied names:

1. Preserve the exact input.
2. Segment it into possible words, names, titles, or suffixes.
3. Generate two to five plausible Chinese candidates.
4. Search each candidate alongside `终末地` or `明日方舟：终末地`.
5. Verify the Chinese form in a reliable source.
6. Cross-check official English and official Vietnamese, if available.
7. Explain the reconstruction briefly and retain unresolved ambiguity.

Return:

```text
Input:
Likely source Chinese:
Official English:
Official Vietnamese:
Generated or Hán-Việt aliases:
Confidence:
Explanation:
Sources:
```

Never persist an attractive guess as an identified entity.

## Special Handling for Enemies

Classify enemies as precisely as the evidence supports:

- Enemy family
- Standard enemy
- Elite
- Boss
- Hostile device or construct

For each enemy, capture:

- Official English and Chinese
- Official Vietnamese if available
- Generated or Hán-Việt aliases
- Family or taxonomy
- Gameplay or lore notes only when sourced
- Community aliases with tone and confidence

Do not infer that a descriptive label is a formal species or family name.

## Special Handling for Plants, Fungi, and Ecology

Terms containing characters such as `菌`, `菌块`, `植株`, `孢子`, or related ecological markers may describe an organism, colony, resource, processed material, hazard, or item. Verify the in-game role before assigning a type.

Capture:

- Official names
- Organism/resource/item classification
- Region or habitat when sourced
- Uses or associated mechanics when sourced
- Community shorthand or aliases

Avoid turning a material derived from a plant or fungus into the organism's proper name without evidence.

## Special Handling for Lore Concepts

For concepts, phenomena, substances, civilizations, and historical terms, distinguish:

- A proper noun from a descriptive phrase
- Official terminology from explanatory translation
- Current terminology from beta or obsolete terminology
- In-world concepts from gameplay mechanics

Include a concise English description only when it can be supported. Avoid filling gaps with fan theories.

## Source-Document Examples

The source document used the following examples to demonstrate format:

- `甚大裂隙` → `Very Large Rift`
- `武陵甚大裂隙` → `Wuling Very Large Rift`
- A Vietnamese rendering equivalent to `Khe Nứt Cực Lớn Võ Lăng`

These examples may be stale, version-specific, unofficial, or otherwise inaccurate. They are **not approved data**. Search current official sources and verify every language before writing any of them to Perwiga.

## Pre-Write Checklist

Before returning or persisting a result, check:

- Official English was verified independently.
- Official Chinese was verified independently.
- Official Vietnamese was searched for rather than inferred.
- Generated Vietnamese is separate from official Vietnamese.
- Hán-Việt and converter forms are labeled and stored as aliases or other information.
- Community terminology is attested and labeled by tone and context.
- Current and legacy names remain distinguishable.
- Entity identity and type were cross-checked.
- Sources, dates, and confidence are attached to assertions.
- No unresolved reverse-lookup candidate is being treated as fact.
- No existing trusted database value will be deleted, cleared, or silently overwritten.
