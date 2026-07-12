from singlesorter_gui.services.singer_list import (
    SingerEntry,
    SingerListStore,
    default_personal_list_path,
)


def test_default_path_honors_explicit_environment(monkeypatch, tmp_path):
    expected = tmp_path / "custom.csv"
    monkeypatch.setenv("SINGLESORTER_PERSONAL_LIST", str(expected))

    assert default_personal_list_path() == expected


def test_store_round_trips_clean_deduplicated_entries(tmp_path):
    store = SingerListStore(tmp_path / "personal.csv")
    entries = [
        SingerEntry("  אברהם פריד ", "אברהם פריד"),
        SingerEntry("אברהם פריד", "אברהם פריד"),
        SingerEntry("", "תיקייה"),
        SingerEntry("מרדכי בן דוד", "מב״ד"),
    ]

    saved = store.save(entries)

    assert saved == 2
    assert store.load() == [
        SingerEntry("אברהם פריד", "אברהם פריד"),
        SingerEntry("מרדכי בן דוד", "מב״ד"),
    ]


def test_import_accepts_utf8_bom_and_merges_existing_rows(tmp_path):
    store = SingerListStore(tmp_path / "personal.csv")
    store.save([SingerEntry("קיים", "קיים")])
    imported = tmp_path / "import.csv"
    imported.write_text("\ufeffחדש,תיקייה חדשה\nקיים,קיים\n", encoding="utf-8")

    count = store.import_csv(imported)

    assert count == 1
    assert store.load()[-1] == SingerEntry("חדש", "תיקייה חדשה")


def test_export_creates_a_portable_utf8_csv(tmp_path):
    store = SingerListStore(tmp_path / "personal.csv")
    store.save([SingerEntry("זמר", "תיקייה")])
    destination = tmp_path / "exported.csv"

    store.export_csv(destination)

    assert destination.read_text(encoding="utf-8-sig").strip() == "זמר,תיקייה"
