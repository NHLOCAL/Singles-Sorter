from pathlib import Path
import sys

import pytest

sys.path.append(str(Path(__file__).resolve().parents[1] / "core"))

from check_name import check_exact_name


@pytest.mark.parametrize(
    ("filename", "artist", "expected"),
    [
        ("מוטי שטיינמץ - שיר חדש", "מוטי שטיינמץ", True),
        ("ועם ומוטי שטיינמץ", "מוטי שטיינמץ", True),
        ("שיר חדש של ויסמנדל", "וייסמנדל", True),
        ("ביצוע של וייסמנדל", "ויסמנדל", True),
        ("אלה קליין - הופעה", "אלי קליין", False),
        ("דואט עם אברהם מרדכי שוורצ", "אברהם מרדכי שוורץ", True),
        ("דואט עם אברהם מורדכי שוורז", "אברהם מרדכי שוורץ", True),
        ("דואט עם אברם מורדכע שוורזז", "אברהם מרדכי שוורץ", False),
        ("שיר נוסף - למוטי שטיינמץ", "מוטי שטיינמץ", False),
        ("שיר נוסף - מוטי שטיינמץל", "מוטי שטיינמץ", False),
        ("יואלי קליין - הופעה", "אלי קליין", False),
        ("ביצוע חי - אלי קלינן", "אלי קליין", True),
        ("הופעה של מרדכי", "אברהם מרדכי שוורץ", False),
    ],
)
def test_check_exact_name_similarity(filename: str, artist: str, expected: bool) -> None:
    assert check_exact_name(filename, artist) is expected
