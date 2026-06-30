#!/usr/bin/env python3
"""Build the visual style bible and prompts for No Future Without Forgiveness."""

from __future__ import annotations

import json
from pathlib import Path


BOOK_FOLDER = "001\u6ca1\u6709\u5bbd\u6055\u5c31\u6ca1\u6709\u672a\u6765"


TERMS = {
    "\u4eba\u7269/\u7fa4\u4f53": [
        "\u56fe\u56fe",
        "\u66fc\u5fb7\u62c9",
        "\u6bcd\u4eb2",
        "\u5b69\u5b50",
        "\u513f\u5b50",
        "\u59bb\u5b50",
        "\u5e78\u5b58\u8005",
        "\u52a0\u5bb3\u8005",
        "\u53d7\u5bb3\u8005",
        "\u8b66\u5bdf",
        "\u58eb\u5175",
        "\u767d\u4eba",
        "\u9ed1\u4eba",
        "\u666e\u901a\u4eba",
        "\u65c1\u89c2\u8005",
        "\u516c\u6c11",
        "\u4e3b\u6559",
        "\u542c\u4f17",
        "\u5bb6\u5ead",
    ],
    "\u5730\u70b9/\u7a7a\u95f4": [
        "\u5357\u975e",
        "\u542c\u8bc1\u4f1a",
        "\u59d4\u5458\u4f1a",
        "\u56fd\u5bb6",
        "\u5bb6\u5ead",
        "\u8857\u5934",
        "\u5b66\u6821",
        "\u6559\u5802",
        "\u76d1\u72f1",
        "\u6cd5\u5ead",
        "\u793e\u533a",
        "\u57ce\u5e02",
        "\u8d2b\u56f0\u57ce\u9547",
        "\u623f\u95f4",
        "\u7a97\u53e3",
        "\u6295\u7968\u7ad9",
        "\u6559\u5802\u9057\u5740",
        "\u5362\u65fa\u8fbe",
    ],
    "\u7269\u54c1/\u9053\u5177": [
        "\u4e66",
        "\u684c\u5b50",
        "\u6905\u5b50",
        "\u7eb8",
        "\u6863\u6848",
        "\u8bc1\u8bcd",
        "\u7167\u7247",
        "\u65e7\u7167\u7247",
        "\u6536\u97f3\u673a",
        "\u62a5\u7eb8",
        "\u7a97\u6237",
        "\u706f",
        "\u9ea6\u514b\u98ce",
        "\u6295\u7968\u7bb1",
        "\u9009\u7968",
        "\u901a\u884c\u8bc1",
        "\u6cd5\u5f8b\u6587\u4ef6",
        "\u9aa8\u7070",
        "\u8863\u7269",
        "\u95e8",
        "\u8def",
        "\u70db\u5149",
        "\u5f55\u97f3\u673a",
        "\u7eb8\u5dfe",
        "\u690d\u7269",
        "\u8336\u676f",
    ],
    "\u81ea\u7136/\u5929\u6c14": [
        "\u591c\u665a",
        "\u9633\u5149",
        "\u9ed1\u591c",
        "\u5149",
        "\u7a7a\u6c14",
        "\u96e8",
        "\u98ce",
        "\u5c18\u571f",
        "\u9053\u8def",
        "\u6811",
    ],
    "\u62bd\u8c61\u4e3b\u9898": [
        "\u771f\u76f8",
        "\u5bbd\u6055",
        "\u548c\u89e3",
        "\u4f24\u75db",
        "\u4ec7\u6068",
        "\u672a\u6765",
        "\u6c89\u9ed8",
        "\u8bb0\u5fc6",
        "\u6b63\u4e49",
        "\u8d23\u4efb",
        "\u4fee\u590d",
        "\u5c0a\u4e25",
        "\u66b4\u529b",
        "\u8c0e\u8a00",
        "\u5e0c\u671b",
        "\u81ea\u7531",
    ],
}


SCENES = [
    {
        "index": 1,
        "time": "00:00-03:45",
        "theme": "\u591c\u8bfb\u5f00\u573a\uff1a\u56fe\u56fe\u4e0e\u95ee\u9898",
        "visual": "\u6e29\u6696\u591c\u665a\u4e66\u684c\uff0c\u4e3b\u6301\u4eba\u89c6\u89d2\u6253\u5f00\u4e66\uff0c\u7a97\u5916\u6697\u84dd\uff0c\u684c\u4e0a\u6709\u53f0\u706f\u3001\u4e66\u3001\u4fbf\u7b7e\u548c\u5357\u975e\u5730\u56fe\u526a\u5f71",
        "visual_en": "A quiet night reading desk, not an open book page: a closed book as a small prop, warm desk lamp, tea cup, handwritten notes shown as blank paper, a simple South Africa map silhouette on the wall, dark blue window outside.",
        "people": "\u4e00\u4f4d\u5b89\u9759\u8bfb\u8005\u7684\u80cc\u5f71\u6216\u624b\u90e8\uff0c\u4e0d\u51fa\u73b0\u771f\u5b9e\u540d\u4eba\u8096\u50cf",
        "people_en": "One quiet reader seen from behind or only hands, no celebrity portrait.",
        "objects": ["\u4e66", "\u53f0\u706f", "\u7a97\u6237", "\u4fbf\u7b7e", "\u5730\u56fe", "\u8336\u676f"],
        "mood": "\u514b\u5236\u3001\u5b89\u9759\u3001\u8fdb\u5165\u5386\u53f2",
    },
    {
        "index": 2,
        "time": "03:45-07:30",
        "theme": "\u79cd\u65cf\u9694\u79bb\u4f5c\u4e3a\u5236\u5ea6\u673a\u5668",
        "visual": "\u57ce\u5e02\u8857\u9053\u88ab\u6805\u680f\u548c\u901a\u884c\u8bc1\u5206\u5272\uff0c\u8fdc\u5904\u8b66\u8f66\u548c\u68c0\u67e5\u70b9\uff0c\u4eba\u7fa4\u6392\u961f\uff0c\u753b\u9762\u4e0d\u8840\u8165",
        "visual_en": "A late-20th-century South African city street divided by fences and checkpoint barriers, a police car in the distance, people queuing with blank pass documents, no violence or gore.",
        "people": "\u4e0d\u540c\u80a4\u8272\u7684\u666e\u901a\u5bb6\u5ead\u88ab\u5236\u5ea6\u9694\u5f00",
        "people_en": "Ordinary families of different skin colors separated by the system, mid-distance view.",
        "objects": ["\u901a\u884c\u8bc1", "\u6805\u680f", "\u8b66\u8f66", "\u65e0\u5b57\u8def\u724c", "\u6587\u4ef6\u5939"],
        "mood": "\u538b\u8feb\u3001\u51b7\u9759\u3001\u5386\u53f2\u611f",
    },
    {
        "index": 3,
        "time": "07:30-11:15",
        "theme": "\u8f6c\u6298\uff1a\u66fc\u5fb7\u62c9\u3001\u56fe\u56fe\u4e0e\u65b0\u5357\u975e",
        "visual": "\u6e05\u6668\u6295\u7968\u7ad9\u5916\u6392\u961f\uff0c\u6559\u5802\u949f\u697c\u8fdc\u666f\uff0c\u9633\u5149\u7167\u8fdb\u4eba\u7fa4\uff0c\u8c61\u5f81\u516c\u6c11\u8eab\u4efd\u6062\u590d",
        "visual_en": "Morning outside a polling station, a long queue of citizens, a church bell tower far away, sunlight entering the crowd, ballot box and blank ballots visible, hopeful civic atmosphere.",
        "people": "\u6392\u961f\u6295\u7968\u7684\u666e\u901a\u4eba\uff0c\u8fdc\u666f\u4e3b\u6559\u8f6e\u5ed3\u4e0d\u753b\u8096\u50cf",
        "people_en": "Ordinary voters in line, a distant symbolic bishop silhouette but no real portrait.",
        "objects": ["\u6295\u7968\u7bb1", "\u9009\u7968", "\u961f\u5217", "\u949f\u697c", "\u6668\u5149"],
        "mood": "\u5e84\u91cd\u3001\u5e0c\u671b",
    },
    {
        "index": 4,
        "time": "11:15-15:00",
        "theme": "\u771f\u76f8\u4e0e\u548c\u89e3\u59d4\u5458\u4f1a",
        "visual": "\u516c\u5171\u542c\u8bc1\u4f1a\u5927\u5385\uff0c\u4e00\u5f20\u684c\u5b50\u3001\u9ea6\u514b\u98ce\u3001\u5f55\u97f3\u8bbe\u5907\uff0c\u5bb6\u5c5e\u5750\u5728\u542c\u4f17\u5e2d\uff0c\u7a7a\u6905\u5b50\u8c61\u5f81\u7f3a\u5e2d\u8005",
        "visual_en": "Inside a public truth and reconciliation hearing hall: a simple testimony table, microphones, an old tape recorder, archive boxes, family members seated in the audience, one empty chair symbolizing the absent.",
        "people": "\u53d1\u8a00\u8005\u3001\u5bb6\u5c5e\u3001\u8bb0\u5f55\u5458\uff0c\u8868\u60c5\u542b\u84c4",
        "people_en": "One speaker at the table, family members listening, one recorder taking notes, restrained facial expressions.",
        "objects": ["\u9ea6\u514b\u98ce", "\u5f55\u97f3\u673a", "\u6863\u6848\u76d2", "\u7a7a\u6905\u5b50", "\u7eb8\u5dfe"],
        "mood": "\u75db\u82e6\u4f46\u6709\u79e9\u5e8f",
    },
    {
        "index": 5,
        "time": "15:00-18:45",
        "theme": "\u53d7\u5bb3\u8005\u9700\u8981\u540d\u5b57\u3001\u4e8b\u5b9e\u548c\u627f\u8ba4",
        "visual": "\u6bcd\u4eb2\u5728\u684c\u524d\u770b\u65e7\u7167\u7247\uff0c\u65c1\u8fb9\u6709\u6863\u6848\u888b\u548c\u4e00\u53ea\u7a7a\u7897\uff0c\u7a97\u5916\u96e8\u505c\u540e\u5fae\u5149",
        "visual_en": "A mother or family member at a modest table looking at an old photo, an archive envelope, one empty bowl, window with rain marks and soft light after rain.",
        "people": "\u4e00\u4f4d\u6bcd\u4eb2\u6216\u5bb6\u5c5e\uff0c\u4e0d\u717d\u60c5",
        "people_en": "One grieving mother or family member, quiet and restrained, not melodramatic.",
        "objects": ["\u65e7\u7167\u7247", "\u6863\u6848\u888b", "\u7a7a\u7897", "\u7a97\u6237", "\u96e8\u75d5"],
        "mood": "\u54c0\u4f24\u3001\u7b49\u5f85\u771f\u76f8",
    },
    {
        "index": 6,
        "time": "18:45-22:30",
        "theme": "\u5bbd\u6055\u4e0d\u662f\u547d\u4ee4\uff0c\u800c\u662f\u53ef\u80fd\u6027",
        "visual": "\u4e24\u6761\u8def\u5728\u5c71\u5761\u5206\u53c9\uff0c\u4e00\u8fb9\u9634\u5f71\u5199\u610f\u6210\u4ec7\u6068\u5faa\u73af\uff0c\u4e00\u8fb9\u901a\u5411\u793e\u533a\u91cd\u5efa\uff1b\u4eba\u7269\u4fdd\u6301\u8ddd\u79bb\uff0c\u6ca1\u6709\u5f3a\u884c\u63e1\u624b",
        "visual_en": "Two roads divide on a hillside; one side falls into shadow suggesting revenge, the other leads toward a small rebuilding community with trees and benches; no forced handshake.",
        "people": "\u4e24\u4e2a\u666e\u901a\u4eba\u7ad9\u5728\u4e0d\u540c\u8ddd\u79bb\uff0c\u65c1\u8fb9\u6709\u65c1\u89c2\u8005\u503e\u542c",
        "people_en": "Two ordinary people standing at a respectful distance, a listener nearby, all in mid-distance.",
        "objects": ["\u9053\u8def", "\u77f3\u5899", "\u793e\u533a\u623f\u5c4b", "\u6811", "\u957f\u6905"],
        "mood": "\u8270\u96be\u3001\u9009\u62e9",
    },
    {
        "index": 7,
        "time": "22:30-26:15",
        "theme": "\u5362\u65fa\u8fbe\u4e0e\u4eba\u6027\u6df1\u5904\u7684\u8b66\u9192",
        "visual": "\u6559\u5802\u9057\u5740\u5916\u7684\u7eaa\u5ff5\u7a7a\u95f4\uff0c\u8721\u70db\u3001\u767d\u5e03\u3001\u788e\u77f3\u548c\u8fdc\u5904\u6811\u5f71\uff0c\u4e0d\u753b\u5c38\u4f53\u548c\u8840\u8165",
        "visual_en": "A respectful memorial space outside a church ruin: candles, white cloth, stones, distant tree shadows, mourning atmosphere, no bodies and no blood.",
        "people": "\u5c11\u91cf\u60bc\u5ff5\u8005\u80cc\u5f71",
        "people_en": "A few mourners seen from behind.",
        "objects": ["\u8721\u70db", "\u767d\u5e03", "\u788e\u77f3", "\u6811\u5f71", "\u65e0\u5b57\u7eaa\u5ff5\u5899"],
        "mood": "\u6c89\u91cd\u3001\u5c0a\u91cd",
    },
    {
        "index": 8,
        "time": "26:15-30:00",
        "theme": "\u56de\u5230\u4e2a\u4eba\u751f\u6d3b\uff1a\u771f\u76f8\u3001\u4fee\u590d\u4e0e\u672a\u6765",
        "visual": "\u6e05\u6668\u623f\u95f4\u5f00\u7a97\uff0c\u4e66\u5408\u4e0a\uff0c\u5149\u7167\u5728\u684c\u4e0a\uff0c\u5bb6\u4eba\u56f4\u5750\u4f46\u7559\u6709\u7a7a\u95f4\uff0c\u8fdc\u5904\u9053\u8def\u5411\u524d",
        "visual_en": "A morning room with an open window, a closed book on the desk, light on the table, family members sitting with gentle space between them, a distant road visible outside.",
        "people": "\u8bfb\u8005\u3001\u5bb6\u4eba\u6216\u542c\u4f17\uff0c\u4fa7\u80cc\u9762\u4e3a\u4e3b",
        "people_en": "Reader and family or listeners, mostly side or back view.",
        "objects": ["\u4e66", "\u7a97\u6237", "\u5149", "\u9053\u8def", "\u8336\u676f", "\u690d\u7269"],
        "mood": "\u6e29\u67d4\u3001\u4fee\u590d\u3001\u5f00\u653e",
    },
]


STYLE = {
    "name": "No Future Without Forgiveness editorial storybook visual system v1",
    "format": "16:9; final video 1920x1080; leave clean lower area for bilingual subtitles",
    "style": "\u4e13\u4e1a\u8bfb\u4e66\u89c6\u9891\u63d2\u753b\uff0c\u4e0d\u662f\u771f\u4eba\u7167\u7247\uff0c\u4e0d\u662f\u62bd\u8c61\u56fe\u6807\uff0c\u4e5f\u4e0d\u662f\u6253\u5f00\u7684\u4e66\u9875\u6446\u62cd\uff1b\u6e29\u6696\u624b\u7ed8\u7eaa\u5f55\u7247\u5206\u955c\u8d28\u611f\uff0c\u7535\u5f71\u6784\u56fe\uff0c\u4e2d\u8fdc\u666f\u53d9\u4e8b\uff0c\u4eba\u7269\u548c\u73af\u5883\u4e00\u8d77\u8bb2\u6545\u4e8b",
    "palette": [
        "cream paper",
        "warm ochre",
        "muted teal",
        "charcoal gray",
        "dusty red accent",
        "soft morning blue",
    ],
    "recurring_motifs": [
        "\u4e66\u684c\u4e0e\u53f0\u706f",
        "\u7a97\u6237\u4e0e\u5149",
        "\u6863\u6848\u76d2/\u8bc1\u8bcd\u7eb8\u5f20",
        "\u9ea6\u514b\u98ce/\u542c\u8bc1\u4f1a",
        "\u7a7a\u6905\u5b50",
        "\u9053\u8def",
        "\u8721\u70db",
        "\u6295\u7968\u7bb1",
        "\u65e7\u7167\u7247",
        "\u5bb6\u5ead\u9910\u684c",
    ],
    "character_rules": [
        "\u4e0d\u753b\u771f\u5b9e\u540d\u4eba\u8096\u50cf\uff0c\u53ea\u7528\u8c61\u5f81\u6027\u4e3b\u6559\u8f6e\u5ed3\u6216\u666e\u901a\u4eba\u7fa4",
        "\u6bcf\u5f20\u56fe\u6700\u591a\u4e00\u4e2a\u4e3b\u89c6\u89c9\u4eba\u7269\u7ec4\uff0c\u907f\u514d\u91cd\u590d\u4eba\u7269",
        "\u8868\u60c5\u514b\u5236\u3001\u81ea\u7136\uff0c\u907f\u514d\u5938\u5f20\u82e6\u60c5\u548c\u6446\u62cd\u611f",
    ],
    "forbidden": [
        "\u53ef\u8bfb\u6587\u5b57",
        "logo/watermark",
        "\u91cd\u590d\u4e3b\u89d2",
        "\u62bd\u8c61\u5355\u7269\u4f53\u56fe\u6807",
        "\u8840\u8165\u66b4\u529b\u753b\u9762",
        "\u73b0\u4ee3\u624b\u673a\u7535\u8111",
        "\u6253\u5f00\u4e66\u9875",
        "\u4e66\u9875\u4e0a\u7684\u4f2a\u6587\u5b57",
        "\u8fc7\u6697\u80cc\u666f",
        "\u8fc7\u5ea6\u6444\u5f71\u68da\u8096\u50cf",
    ],
}


def find_book_dir() -> Path:
    matches = [path for path in Path("D:/books").rglob(BOOK_FOLDER) if path.is_dir()]
    if not matches:
        raise FileNotFoundError(f"Book directory not found: {BOOK_FOLDER}")
    return matches[0]


def load_narration(book_dir: Path) -> str:
    materials = json.loads((book_dir / "output" / "materials.json").read_text(encoding="utf-8"))
    return str(materials.get("narration") or "")


def count_terms(text: str) -> dict[str, dict[str, int]]:
    return {
        category: {
            term: count
            for term in items
            if (count := text.count(term)) > 0
        }
        for category, items in TERMS.items()
    }


def build_prompt(scene: dict) -> str:
    base_style = (
        "Single frame from a warm hand-painted animated historical documentary; "
        "cinematic mid-shot or wide-shot composition; no book, no open pages, no page layout; "
        "rich but readable details; muted warm earth palette, cream paper highlights, charcoal accents, "
        "subtle teal and dusty red; late-20th-century South Africa historical atmosphere; "
        "serious, humane, hopeful; no readable text, no watermark, no logo, no duplicate protagonist, "
        "no abstract single-object icon, no gore, no open book spread, no page layout, no fake letters."
    )
    return " ".join(
        [
            base_style,
            f"Scene {scene['index']} of 8, time range {scene['time']}.",
            f"Theme: {scene['theme']}.",
            f"Visual composition: {scene['visual_en']}.",
            f"People: {scene['people_en']}.",
            f"Objects to include naturally: {', '.join(scene['objects'])}.",
            f"Mood: {scene['mood']}.",
            "Make it a complete narrative picture with people, place, objects, light, and atmosphere. Leave clean lower area for bilingual subtitles.",
        ]
    )


def scan_mojibake(paths: list[Path]) -> list[str]:
    markers = ["\ufffd", "\u951f", "\u9420", "\u95c1", "???"]
    bad: list[str] = []
    for path in paths:
        text = path.read_text(encoding="utf-8")
        if any(marker in text for marker in markers):
            bad.append(str(path))
    return bad


def main() -> int:
    book_dir = find_book_dir()
    out_dir = book_dir / "output_regen_design_001"
    out_dir.mkdir(exist_ok=True)

    narration = load_narration(book_dir)
    noun_counts = count_terms(narration)
    prompts = [
        {
            "index": scene["index"],
            "time": scene["time"],
            "theme": scene["theme"],
            "prompt": build_prompt(scene),
        }
        for scene in SCENES
    ]

    style = {
        **STYLE,
        "sourceBookDir": str(book_dir),
        "noun_counts": noun_counts,
        "scenes": SCENES,
    }

    visual_json = out_dir / "visual_style_bible.json"
    visual_md = out_dir / "visual_style_bible.md"
    prompts_json = out_dir / "prompts_8.json"
    prompts_md = out_dir / "prompts_8.md"

    visual_json.write_text(json.dumps(style, ensure_ascii=False, indent=2), encoding="utf-8")
    prompts_json.write_text(json.dumps(prompts, ensure_ascii=False, indent=2), encoding="utf-8")

    lines = [
        "# \u300a\u6ca1\u6709\u5bbd\u6055\u5c31\u6ca1\u6709\u672a\u6765\u300b\u89c6\u89c9\u7cfb\u7edf v1",
        "",
        f"\u8f93\u51fa\u76ee\u5f55\uff1a`{out_dir}`",
        "",
        "## \u98ce\u683c\u5b9a\u4f4d",
        STYLE["style"],
        "",
        "## \u56fa\u5b9a\u5143\u7d20",
    ]
    lines.extend(f"- {item}" for item in STYLE["recurring_motifs"])
    lines.extend(["", "## \u540d\u8bcd/\u7269\u54c1\u62bd\u53d6"])
    for category, items in noun_counts.items():
        lines.append(f"### {category}")
        if items:
            lines.extend(f"- {term}: {count}" for term, count in sorted(items.items(), key=lambda item: -item[1]))
        else:
            lines.append("- \u6682\u65e0\u547d\u4e2d")
    lines.extend(["", "## 8 \u6bb5\u65f6\u95f4\u8f74\u5206\u955c"])
    for scene in SCENES:
        lines.extend(
            [
                f"### {scene['index']}. {scene['time']} {scene['theme']}",
                f"- \u753b\u9762\uff1a{scene['visual']}",
                f"- \u4eba\u7269\uff1a{scene['people']}",
                f"- \u7269\u54c1\uff1a{'\u3001'.join(scene['objects'])}",
                f"- \u60c5\u7eea\uff1a{scene['mood']}",
            ]
        )
    visual_md.write_text("\n".join(lines) + "\n", encoding="utf-8")

    prompt_lines = ["# \u51fa\u56fe\u63d0\u793a\u8bcd v1", ""]
    for item in prompts:
        prompt_lines.extend(
            [
                f"## {item['index']}. {item['time']} {item['theme']}",
                item["prompt"],
                "",
            ]
        )
    prompts_md.write_text("\n".join(prompt_lines), encoding="utf-8")

    files = [visual_json, visual_md, prompts_json, prompts_md]
    result = {
        "outDir": str(out_dir),
        "files": [str(path) for path in files],
        "badFiles": scan_mojibake(files),
    }
    print(json.dumps(result, ensure_ascii=False))
    return 1 if result["badFiles"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
