import json

from singlesorter_gui.services.updates import (
    ReleaseInfo,
    check_latest_release,
    parse_version,
    release_from_payload,
    should_show_whats_new,
)
from singlesorter_gui.tips import tip_for_index


def test_semantic_versions_are_compared_numerically():
    assert parse_version("v14.0.10") > parse_version("14.0.2")
    assert parse_version("release-14.1") == (14, 1, 0)


def test_release_payload_ignores_prereleases_and_old_versions():
    prerelease = {"tag_name": "v15.0.0", "prerelease": True}
    current = {"tag_name": "v14.0.1", "prerelease": False}

    assert release_from_payload(prerelease, "14.0.1") is None
    assert release_from_payload(current, "14.0.1") is None


def test_release_client_returns_structured_new_release():
    payload = {
        "tag_name": "v14.1.0",
        "prerelease": False,
        "body": "שיפורים חשובים",
        "html_url": "https://example.test/release",
    }

    class Response:
        def __enter__(self):
            return self

        def __exit__(self, *_):
            return None

        def read(self):
            return json.dumps(payload).encode()

    release = check_latest_release("14.0.1", opener=lambda *_, **__: Response())

    assert release == ReleaseInfo("14.1.0", "שיפורים חשובים", "https://example.test/release")


def test_first_run_and_tip_selection_are_deterministic():
    assert should_show_whats_new(None, "14.0.1") is True
    assert should_show_whats_new("14.0.1", "14.0.1") is False
    assert tip_for_index(0) == tip_for_index(100)
    assert "גיבוי" in tip_for_index(0)
