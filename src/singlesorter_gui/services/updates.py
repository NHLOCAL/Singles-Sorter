"""Small, timeout-bounded release checker with no third-party HTTP dependency."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from typing import Callable
from urllib.request import Request, urlopen

RELEASE_API = "https://api.github.com/repos/NHLOCAL/Singles-Sorter/releases/latest"


@dataclass(frozen=True, slots=True)
class ReleaseInfo:
    version: str
    notes: str
    url: str


def parse_version(value: str) -> tuple[int, int, int]:
    parts = [int(part) for part in re.findall(r"\d+", value)[:3]]
    return tuple((parts + [0, 0, 0])[:3])


def release_from_payload(payload: dict, current_version: str) -> ReleaseInfo | None:
    if payload.get("prerelease"):
        return None
    tag = str(payload.get("tag_name", ""))
    if not tag or parse_version(tag) <= parse_version(current_version):
        return None
    version = ".".join(str(part) for part in parse_version(tag))
    return ReleaseInfo(
        version=version,
        notes=str(payload.get("body") or "לא צורפו פרטי שחרור."),
        url=str(
            payload.get("html_url")
            or "https://github.com/NHLOCAL/Singles-Sorter/releases/latest"
        ),
    )


def check_latest_release(
    current_version: str,
    *,
    opener: Callable = urlopen,
    timeout: float = 5.0,
) -> ReleaseInfo | None:
    request = Request(
        RELEASE_API,
        headers={"Accept": "application/vnd.github+json", "User-Agent": "Singles-Sorter"},
    )
    try:
        with opener(request, timeout=timeout) as response:
            payload = json.loads(response.read().decode("utf-8"))
        return release_from_payload(payload, current_version)
    except (OSError, TimeoutError, ValueError, json.JSONDecodeError):
        return None


def should_show_whats_new(last_seen_version: object, current_version: str) -> bool:
    return str(last_seen_version or "") != current_version
