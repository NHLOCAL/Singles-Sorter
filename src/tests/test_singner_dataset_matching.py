from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

sys.path.append(str(Path(__file__).resolve().parents[1] / "core"))

from check_name import check_exact_name


DATASET_SAMPLE_PATH = Path(__file__).resolve().parent / "data" / "singner_sample_5000.jsonl"

def _load_dataset_sample() -> list[dict]:
    rows: list[dict] = []
    with DATASET_SAMPLE_PATH.open("r", encoding="utf-8") as sample_file:
        for line in sample_file:
            rows.append(json.loads(line))
    return rows


@pytest.fixture(scope="module")
def singner_rows() -> list[dict]:
    rows = _load_dataset_sample()
    assert len(rows) >= 5000, "Expected at least 5000 rows in sample"
    return rows


@pytest.fixture(scope="module")
def singer_mentions(singner_rows: list[dict]) -> list[tuple[str, str]]:
    mentions: list[tuple[str, str]] = []
    for row in singner_rows:
        text = row["text"]
        for entity in row["entities"]:
            if entity.get("label") != "SINGER":
                continue
            singer_name = text[entity["start"]:entity["end"]].strip()
            if singer_name:
                mentions.append((text, singer_name))

    assert mentions, "No singer mentions extracted from dataset"
    return mentions


def _pick_unique_singers(mentions: list[tuple[str, str]], limit: int) -> list[str]:
    seen: set[str] = set()
    singers: list[str] = []

    for _, singer in mentions:
        if singer not in seen:
            seen.add(singer)
            singers.append(singer)
        if len(singers) >= limit:
            break

    return singers


def test_dataset_row_count_is_significant(singner_rows: list[dict]) -> None:
    assert len(singner_rows) == 5000


def test_singer_entities_match_in_original_text(singer_mentions: list[tuple[str, str]]) -> None:
    sampled_mentions = singer_mentions[:1500]
    matched = sum(1 for text, singer in sampled_mentions if check_exact_name(text, singer))
    ratio = matched / len(sampled_mentions)

    assert ratio >= 0.98, f"Unexpectedly low match ratio on dataset sample: {ratio:.3f}"


def test_comprehensive_edge_cases_from_dataset(singer_mentions: list[tuple[str, str]]) -> None:
    unique_singers = _pick_unique_singers(singer_mentions, limit=300)
    assert len(unique_singers) >= 120

    edge_cases: list[tuple[str, str, bool]] = []

    # 1) Positive direct cases from dataset (40)
    for text, singer in singer_mentions[:40]:
        edge_cases.append((text, singer, True))

    # 2) Positive with optional Hebrew vav-prefix (20)
    vav_candidates = [s for s in unique_singers if not s.startswith("ו")]
    for singer in vav_candidates[:20]:
        edge_cases.append((f"שיר חי עם ו{singer}", singer, True))

    # 3) Negative: prefix expansion should not match (20)
    for singer in unique_singers[20:40]:
        edge_cases.append((f"ל{singer} בהופעה", singer, False))

    # 4) Negative: suffix expansion should not match (20)
    for singer in unique_singers[40:60]:
        edge_cases.append((f"{singer}ל בהופעה", singer, False))

    # 5) Negative: text with no realistic artist mention (20)
    for singer in unique_singers[60:80]:
        edge_cases.append(("zzzzq xxyy 12345 ללא זמר מזוהה", singer, False))

    assert len(edge_cases) >= 120

    failed_cases = [
        (filename, artist, expected)
        for filename, artist, expected in edge_cases
        if check_exact_name(filename, artist) is not expected
    ]

    assert not failed_cases, f"Found {len(failed_cases)} failing edge cases"


def test_no_answer_when_artist_not_present(singer_mentions: list[tuple[str, str]]) -> None:
    unique_singers = _pick_unique_singers(singer_mentions, limit=100)
    text_without_singer = "גרסת בדיקה ללא התאמה כלל 987654321"

    assert all(not check_exact_name(text_without_singer, singer) for singer in unique_singers)
