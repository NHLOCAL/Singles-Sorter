"""Characterization of v14.0 released decisions, not a rewritten engine.

All filesystem inputs are disposable placeholders. Metadata/model outputs are
injected where noted; these tests make no claim about real-format or ML parity.
"""
import csv
import logging
from pathlib import Path

import pytest

import singles_sorter_v5 as legacy
from check_name import check_exact_name
from jibrish_to_hebrew import fix_jibrish


class Field:
    def __init__(self, value):
        self.value = value


class Metadata(dict):
    def __init__(self, **values):
        super().__init__({name: Field(values.get(name, "")) for name in
                         ("artist", "albumartist", "title", "album", "genre", "tracknumber")})
        self.saved = 0

    def __setitem__(self, key, value):
        super().__setitem__(key, value if isinstance(value, Field) else Field(value))

    def save(self):
        self.saved += 1


class FakeAI:
    def __init__(self, accepted=True, ner=None):
        self.accepted = accepted
        self.ner = ner or {}
        self.calls = []

    def verify_artist_with_sklearn(self, value):
        self.calls.append(("classify", value))
        return self.accepted

    def process_with_ner(self, value):
        self.calls.append(("ner", value))
        return self.ner.get(value, [])


@pytest.fixture
def make_sorter(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    app = tmp_path / "app"
    app.mkdir()
    (app / "singer-list.csv").write_text("אברהם פריד,אברהם פריד\nמוטי שטיינמץ,מוטי שטיינמץ\n", encoding="utf-8")
    source, target = tmp_path / "source", tmp_path / "target"
    source.mkdir()
    target.mkdir()
    monkeypatch.setattr(legacy, "ai_invalid", True)

    def create(**settings):
        return legacy.MusicSorter(source, target, logger=logging.getLogger("dev-contract"), **settings)
    return create


def song(sorter, name="unrecognized.mp3"):
    path = sorter.source_dir / name
    path.write_bytes(b"synthetic placeholder, not an audio recording")
    return path


def album(sorter, monkeypatch, rows, name="album", child_dirs=0):
    path = sorter.source_dir / name
    path.mkdir()
    values = {}
    for index, row in enumerate(rows):
        file = path / f"track_{chr(97 + index)}.mp3"
        file.write_bytes(b"placeholder")
        values[file] = row
    for index in range(child_dirs):
        (path / f"extra-{index}").mkdir()

    def read(file):
        value = values[file]
        if isinstance(value, Exception):
            raise value
        return value
    monkeypatch.setattr(legacy, "load_file", read)
    return path


def test_defaults_are_release_defaults(make_sorter):
    value = make_sorter()
    assert (value.copy_mode, value.abc_sort, value.exist_only, value.singles_folder,
            value.main_folder_only, value.duet_mode) == (False, False, False, True, False, False)
    assert value.ai_models is None
    assert not hasattr(value, "cancel_check")


@pytest.mark.parametrize("copy,abc,singles,parts", [
    (False, False, False, ("אברהם פריד",)),
    (True, False, True, ("אברהם פריד", "סינגלים")),
    (False, True, False, ("א", "אברהם פריד")),
    (True, True, True, ("א", "אברהם פריד", "סינגלים")),
])
def test_target_paths(make_sorter, copy, abc, singles, parts):
    s = make_sorter(copy_mode=copy, abc_sort=abc, singles_folder=singles)
    assert s.get_target_path("אברהם פריד") == s.target_dir.joinpath(*parts)


@pytest.mark.parametrize("text,expected", [
    ("אבי_בדיקה.mp3", "אבי בדיקה.mp3"),
    ("אבי_בדיקה - שיר.mp3", "אביבדיקה - שיר.mp3"),
    ("אבי-בדיקה.mp3", "אבי בדיקה.mp3"),
    ("מייל מיוזיק ציצו במייל שיר.mp3", "שיר.mp3"),
    ("  שיר\tחדש  .mp3 ", "שיר חדש .mp3"),
])
def test_cleanup(make_sorter, text, expected):
    assert make_sorter().clean_filename(text) == expected


@pytest.mark.parametrize("text,expected", [
    ('a<>:"/\\|?*b.mp3', "ab.mp3"),
    ("   ", None),
    ("CON.mp3", "CON.mp3"),
    ("name.", "name."),
    ("a" * 260 + ".mp3", "a" * 255),
])
def test_filename_sanitation(make_sorter, text, expected):
    assert make_sorter().sanitize_filename(text) == expected


@pytest.mark.parametrize("text,mode,expected", [
    ("abc àא", "auto", "abc אא"),
    ("abc אà", "auto", "abc àà"),
    ("àìáåí", "heb", "אלבום"),
    ("אלבום", "jib", "àìáåí"),
    ("", "heb", ""),
])
def test_encoding(text, mode, expected):
    assert fix_jibrish(text, mode) == expected


@pytest.mark.parametrize("name,expected", [
    ("אברהם פריד", True), ("דוד", True), ("אברהם מרדכי שוורץ", True),
    ("אברהם מרדכי שוורץ זמר", False), ("סינגלים", False), ("לא ידוע", False),
    ("John Smith", False), ("אברהם\tפריד", False), ("דוד׳", True),
    ("דוד״", False), ("דָוד", False), ("", False),
])
def test_candidate_validation(make_sorter, name, expected):
    assert make_sorter().check_artist(name) is expected


def test_csv_order_duplicates_and_whitespace_are_preserved(make_sorter, tmp_path):
    s = make_sorter()
    (tmp_path / "app/personal-singer-list.csv").write_text("אברהם פריד,אחר\n שם , יעד \n", encoding="utf-8")
    assert s.list_from_csv() == [("אברהם פריד", "אברהם פריד"), ("מוטי שטיינמץ", "מוטי שטיינמץ"),
                                 ("אברהם פריד", "אחר"), (" שם ", " יעד ")]


@pytest.mark.parametrize("duet", [False, True])
def test_filename_list_stops_at_first_row_even_for_duets(make_sorter, monkeypatch, duet):
    s = make_sorter(duet_mode=duet)
    p = song(s, "מוטי שטיינמץ ואברהם פריד.mp3")
    monkeypatch.setattr(legacy, "load_file", lambda _: Metadata(artist="מוטי שטיינמץ", title="מוטי שטיינמץ"))
    assert s.artists_from_song(p) == ["אברהם פריד"]


def test_exact_matching_skips_earlier_nonexact_row(make_sorter, monkeypatch):
    s = make_sorter()
    s.singer_list = [("ויסמנדל", "first"), ("וייסמנדל", "second")]
    monkeypatch.setattr(legacy, "load_file", lambda _: Metadata())
    assert s.artists_from_song(song(s, "וייסמנדל - שיר.mp3")) == ["second"]


def test_metadata_artist_precedes_title(make_sorter, monkeypatch):
    s = make_sorter()
    monkeypatch.setattr(legacy, "load_file", lambda _: Metadata(artist="מוטי שטיינמץ", title="אברהם פריד"))
    assert s.artists_from_song(song(s)) == ["מוטי שטיינמץ"]


def test_filename_result_survives_metadata_read_error(make_sorter, monkeypatch):
    s = make_sorter()
    def fail(_): raise OSError("injected")
    monkeypatch.setattr(legacy, "load_file", fail)
    assert s.artists_from_song(song(s, "אברהם פריד.mp3")) == ["אברהם פריד"]


def test_unknown_metadata_artist_requires_ai(make_sorter, monkeypatch):
    s = make_sorter()
    monkeypatch.setattr(legacy, "load_file", lambda _: Metadata(artist="דוד ישראל"))
    assert s.artists_from_song(song(s)) is None


def test_classifier_artist_precedes_title(make_sorter, monkeypatch):
    s = make_sorter()
    s.ai_models = FakeAI()
    monkeypatch.setattr(legacy, "ai_invalid", False)
    monkeypatch.setattr(legacy, "load_file", lambda _: Metadata(artist="דוד ישראל", title="אברהם פריד"))
    assert s.artists_from_song(song(s)) == ["דוד ישראל"]
    assert s.ai_models.calls == [("classify", "דוד ישראל")]


def test_classifier_rejection_falls_through_to_title(make_sorter, monkeypatch):
    s = make_sorter()
    s.ai_models = FakeAI(accepted=False)
    monkeypatch.setattr(legacy, "ai_invalid", False)
    monkeypatch.setattr(legacy, "load_file", lambda _: Metadata(artist="דוד ישראל", title="אברהם פריד"))
    assert s.artists_from_song(song(s)) == ["אברהם פריד"]


def test_ner_filename_precedes_title_and_preserves_duplicates(make_sorter, monkeypatch):
    s = make_sorter()
    s.ai_models = FakeAI(ner={"unrecognized": ["English Name", "English Name"]})
    monkeypatch.setattr(legacy, "ai_invalid", False)
    monkeypatch.setattr(legacy, "load_file", lambda _: Metadata(title="unknown title"))
    assert s.artists_from_song(song(s)) == ["English Name", "English Name"]
    assert s.ai_models.calls == [("ner", "unrecognized")]


def test_ner_title_only_after_empty_filename_ner(make_sorter, monkeypatch):
    s = make_sorter()
    s.ai_models = FakeAI(ner={"unknown title": ["דוד ישראל"]})
    monkeypatch.setattr(legacy, "ai_invalid", False)
    monkeypatch.setattr(legacy, "load_file", lambda _: Metadata(title="unknown title"))
    assert s.artists_from_song(song(s)) == ["דוד ישראל"]
    assert s.ai_models.calls == [("ner", "unrecognized"), ("ner", "unknown title")]


@pytest.mark.parametrize("count,expected", [(2, False), (3, True)])
def test_album_minimum_files(make_sorter, monkeypatch, count, expected):
    s = make_sorter()
    p = album(s, monkeypatch, [Metadata(album="אלבום", artist="אברהם פריד") for _ in range(count)])
    assert s.analyze_album(p)[0] is expected


@pytest.mark.parametrize("children,expected", [(0, True), (1, True), (2, False)])
def test_album_child_directory_limit(make_sorter, monkeypatch, children, expected):
    s = make_sorter()
    p = album(s, monkeypatch, [Metadata(album="אלבום", artist="אברהם פריד") for _ in range(3)], child_dirs=children)
    assert s.analyze_album(p)[0] is expected


@pytest.mark.parametrize("matching,expected", [(6, False), (7, True), (8, True)])
def test_album_seventy_percent_threshold(make_sorter, monkeypatch, matching, expected):
    s = make_sorter()
    rows = [Metadata(album="אלבום", artist="אברהם פריד") for _ in range(matching)]
    rows += [Metadata(album=f"אחר-{i}", artist="אברהם פריד") for i in range(10-matching)]
    assert s.analyze_album(album(s, monkeypatch, rows))[0] is expected


def test_unreadable_files_remain_in_album_denominator(make_sorter, monkeypatch):
    s = make_sorter()
    rows = [Metadata(album="אלבום", artist="אברהם פריד") for _ in range(2)] + [OSError("injected")]
    assert s.analyze_album(album(s, monkeypatch, rows))[0] is False


def test_no_readable_metadata_protects_folder(make_sorter, monkeypatch):
    s = make_sorter()
    assert s.analyze_album(album(s, monkeypatch, [OSError("injected") for _ in range(3)])) == (True, False, None, None)


def test_blank_artists_excluded_from_artist_denominator(make_sorter, monkeypatch):
    s = make_sorter()
    rows = [Metadata(album="אלבום", artist="אברהם פריד"), Metadata(album="אלבום"), Metadata(album="אלבום")]
    assert s.analyze_album(album(s, monkeypatch, rows)) == (True, True, "אלבום", "אברהם פריד")


def test_albumartist_wins_over_artist(make_sorter, monkeypatch):
    s = make_sorter()
    rows = [Metadata(album="אלבום", artist="אברהם פריד", albumartist="מוטי שטיינמץ") for _ in range(3)]
    assert s.analyze_album(album(s, monkeypatch, rows))[3] == "מוטי שטיינמץ"


@pytest.mark.parametrize("field,value", [("album", "Super Singles"), ("artist", "זמר סינגלים")])
def test_single_keywords_override_album(make_sorter, monkeypatch, field, value):
    s = make_sorter()
    rows = [Metadata(album="אלבום", artist="אברהם פריד", tracknumber=str(i+1)) for i in range(3)]
    rows[0][field] = value
    assert s.analyze_album(album(s, monkeypatch, rows))[0] is False


def test_single_keyword_folder_name_does_not_override_album(make_sorter, monkeypatch):
    s = make_sorter()
    rows = [Metadata(album="אלבום", artist="אברהם פריד") for _ in range(3)]
    assert s.analyze_album(album(s, monkeypatch, rows, name="סינגלים"))[0] is True


@pytest.mark.parametrize("tracks,expected", [([1, 2, 3], True), ([1, 1, 3], True), ([1, 2, 4], False), ([0, 1, 2], False)])
def test_track_sequence_contract(make_sorter, monkeypatch, tracks, expected):
    s = make_sorter()
    rows = [Metadata(artist="אברהם פריד", tracknumber=str(x)) for x in tracks]
    assert s.analyze_album(album(s, monkeypatch, rows))[0] is expected


def test_album_without_artist_is_protected(make_sorter, monkeypatch):
    s = make_sorter()
    rows = [Metadata(album="אלבום") for _ in range(3)]
    assert s.analyze_album(album(s, monkeypatch, rows)) == (True, False, "אלבום", None)


def test_album_summary_preserves_release_counting(make_sorter):
    s = make_sorter(copy_mode=True)
    p = s.source_dir / "album"
    p.mkdir()
    (p / "song.mp3").write_bytes(b"placeholder")
    (p / "cover.jpg").write_bytes(b"placeholder")
    s.handle_album_transfer(p, "אלבום", "אברהם פריד")
    assert s.generate_summary() == {"songs_sorted": 0, "artist_folders_created": 0, "albums_processed": 1,
                                    "top_artists": [("אברהם פריד", 2)]}


def test_existing_album_skipped_as_a_whole(make_sorter):
    s = make_sorter(copy_mode=True)
    p = s.source_dir / "album"
    p.mkdir()
    (p / "song.mp3").write_bytes(b"placeholder")
    (s.target_dir / "אברהם פריד/אלבום").mkdir(parents=True)
    s.handle_album_transfer(p, "אלבום", "אברהם פריד")
    assert s.albums_processed == 0
    assert not (s.target_dir / "אברהם פריד/אלבום/song.mp3").exists()


def test_main_only_still_processes_direct_child_files(make_sorter, monkeypatch):
    s = make_sorter(copy_mode=True, main_folder_only=True, singles_folder=False)
    p = s.source_dir / "child"
    p.mkdir()
    (p / "אברהם פריד.mp3").write_bytes(b"placeholder")
    monkeypatch.setattr(legacy, "load_file", lambda _: Metadata())
    assert s.scan_dir()["songs_sorted"] == 1
    assert (s.target_dir / "אברהם פריד/אברהם פריד.mp3").exists()


def test_sort_uses_original_filename_not_recognition_cleaned_name(make_sorter, monkeypatch):
    s = make_sorter(copy_mode=True, singles_folder=False)
    p = song(s, "מייל מיוזיק אברהם פריד.mp3")
    monkeypatch.setattr(legacy, "load_file", lambda _: Metadata())
    s.scan_dir()
    assert (s.target_dir / "אברהם פריד" / p.name).exists()


def test_fix_names_repairs_five_tags_but_not_filename_gibberish(make_sorter, monkeypatch):
    s = make_sorter()
    p = song(s, "àìáåí.mp3")
    values = Metadata(artist="à", albumartist="à", title="à", album="à", genre="à", tracknumber="1")
    monkeypatch.setattr(legacy, "load_file", lambda _: values)
    s.fix_names()
    assert p.exists()
    assert [values[k].value for k in ["artist", "albumartist", "title", "album", "genre"]] == ["א"] * 5
    assert values["tracknumber"].value == "1"
    assert values.saved == 1
