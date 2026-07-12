from singlesorter_gui.services.sorting import CancellationToken, SortService
from singlesorter_gui.state import SortJob, SortSettings


class FakeSorter:
    def __init__(self, **kwargs):
        self.kwargs = kwargs

    def scan_dir(self):
        self.kwargs["progress_callback"](25)
        self.kwargs["progress_callback"](25)
        self.kwargs["progress_callback"](100)
        return {
            "songs_sorted": 7,
            "artist_folders_created": 3,
            "albums_processed": 1,
            "top_artists": [("אמן", 4)],
        }

    def fix_names(self):
        self.kwargs["progress_callback"](50)
        self.kwargs["progress_callback"](100)


def test_service_maps_safe_settings_and_deduplicates_progress(tmp_path):
    created = []
    progress = []

    def factory(**kwargs):
        sorter = FakeSorter(**kwargs)
        created.append(sorter)
        return sorter

    job = SortJob(
        source=tmp_path / "source",
        target=tmp_path / "target",
        settings=SortSettings(copy_mode=True, abc_sort=True),
    )
    result = SortService(sorter_factory=factory).run(job, progress.append)

    assert created[0].kwargs["copy_mode"] is True
    assert created[0].kwargs["abc_sort"] is True
    assert [item.fraction for item in progress] == [0.25, 1.0]
    assert result.songs_sorted == 7
    assert result.artist_folders_created == 3


def test_cancellation_token_is_cooperative():
    token = CancellationToken()
    assert token.cancelled is False
    token.cancel()
    assert token.cancelled is True


def test_fix_names_uses_source_only_and_reports_completion(tmp_path):
    created = []
    progress = []

    def factory(**kwargs):
        instance = FakeSorter(**kwargs)
        created.append(instance)
        return instance

    result = SortService(sorter_factory=factory).fix_names(tmp_path, progress.append)

    assert created[0].kwargs["source_dir"] == tmp_path
    assert created[0].kwargs["target_dir"] is None
    assert [item.fraction for item in progress] == [0.5, 1.0]
    assert result.message == "תיקון שמות הקבצים הסתיים בהצלחה."
