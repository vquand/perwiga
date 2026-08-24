# Region emblem assets

These seven files are the Genshin in-game region emblems in their white-on-transparent form. Perwiga uses them only as compact list context; the official HoYoverse character directory remains the source of truth for each Character's region affiliation.

The files were fetched on 2026-08-24 from the public Genshin Impact Wiki region-emblem catalog. Its media host content-negotiated six PNG catalog entries to lossless WebP and Natlan to WebP with alpha, so the stored extension and served media type are `.webp` / `image/webp`. The app bundles the bytes locally and makes no runtime request to the wiki.

| Region | Catalog entry | SHA-256 |
| --- | --- | --- |
| Mondstadt | <https://static.wikia.nocookie.net/gensin-impact/images/9/99/Emblem_Mondstadt_White.png/revision/latest?cb=20220301033214> | `3b7e58e9e16c0e5fe3303f754d7aea93c55253dbeb9c5a6ba416e1fa67099e9c` |
| Liyue | <https://static.wikia.nocookie.net/gensin-impact/images/4/49/Emblem_Liyue_White.png/revision/latest?cb=20220301033230> | `5f02e1b05f27d15a8609da5d5f95df1a8a654941e37808dda9b9bccb43e606b0` |
| Inazuma | <https://static.wikia.nocookie.net/gensin-impact/images/5/51/Emblem_Inazuma_White.png/revision/latest?cb=20220301030931> | `1bf9a2b44f33067291580e7c62625f62f85134a72c365e0bfeecc2c9b535306f` |
| Sumeru | <https://static.wikia.nocookie.net/gensin-impact/images/6/6a/Emblem_Sumeru_White.png/revision/latest?cb=20220718184158> | `f197e25e565b6a9b987a2998493750c1f91bee8bcff1d1de999f809d5fba84ad` |
| Fontaine | <https://static.wikia.nocookie.net/gensin-impact/images/7/7b/Emblem_Fontaine_White.png/revision/latest?cb=20230807032406> | `57c05b37e809d2e7bd6302215d4743668affc17c74d97a3dfca5166eed9f1ec1` |
| Natlan | <https://static.wikia.nocookie.net/gensin-impact/images/1/10/Emblem_Natlan_White.png/revision/latest?cb=20240828024938> | `c95ca9e305eaa525eb912dd8d67d810eead7b71ced4f8bfe361e3069376c5dda` |
| Snezhnaya | <https://static.wikia.nocookie.net/gensin-impact/images/8/8d/Emblem_Snezhnaya_White.png/revision/latest?cb=20260813033759> | `62ac605245eac603a53e3797cd78364a70a452d51f2e95dad4c632035e627986` |

Catalog API used for review: <https://genshin-impact.fandom.com/api.php?action=query&generator=categorymembers&gcmtitle=Category%3ARegion%20Emblems&gcmtype=file&gcmlimit=100&prop=imageinfo&iiprop=url%7Csize&format=json>

These community-indexed game assets are presentation-only and must not be used as naming, localization, or character-affiliation evidence.
