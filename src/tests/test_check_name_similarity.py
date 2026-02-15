import sys
import unittest
from pathlib import Path

sys.path.append(str(Path(__file__).resolve().parents[1] / "core"))

from check_name import check_exact_name


class TestCheckNameSimilarity(unittest.TestCase):
    def test_exact_match_still_supported(self):
        self.assertTrue(check_exact_name("מוטי שטיינמץ - שיר חדש", "מוטי שטיינמץ"))

    def test_vav_prefix_still_supported(self):
        self.assertTrue(check_exact_name("ועם ומוטי שטיינמץ", "מוטי שטיינמץ"))

    def test_one_letter_difference_in_word_is_supported(self):
        self.assertTrue(check_exact_name("שיר חדש של ויסמנדל", "וייסמנדל"))
        self.assertTrue(check_exact_name("ביצוע של וייסמנדל", "ויסמנדל"))

    def test_short_name_does_not_match_with_one_letter_change(self):
        self.assertFalse(check_exact_name("אלה קליין - הופעה", "אלי קליין"))

    def test_long_name_supports_two_differences(self):
        self.assertTrue(
            check_exact_name(
                "דואט עם אברהם מרדכי שוורצ",  # חסרה אות אחת במילה האחרונה
                "אברהם מרדכי שוורץ",
            )
        )
        self.assertTrue(
            check_exact_name(
                "דואט עם אברהם מורדכי שוורז",  # 2 הבדלים מפוזרים בשם ארוך
                "אברהם מרדכי שוורץ",
            )
        )

    def test_long_name_rejects_when_difference_too_large(self):
        self.assertFalse(
            check_exact_name(
                "דואט עם אברם מורדכע שוורזז",
                "אברהם מרדכי שוורץ",
            )
        )

    def test_prevent_false_positive_for_prefix_forms(self):
        self.assertFalse(check_exact_name("שיר נוסף - למוטי שטיינמץ", "מוטי שטיינמץ"))

    def test_prevent_false_positive_for_suffix_forms(self):
        self.assertFalse(check_exact_name("שיר נוסף - מוטי שטיינמץל", "מוטי שטיינמץ"))

    def test_prevent_false_positive_for_distinct_names(self):
        self.assertFalse(check_exact_name("יואלי קליין - הופעה", "אלי קליין"))

    def test_multi_word_window_matching(self):
        self.assertTrue(check_exact_name("ביצוע חי - אלי קלינן", "אלי קליין"))

    def test_window_requires_same_word_count(self):
        self.assertFalse(check_exact_name("הופעה של מרדכי", "אברהם מרדכי שוורץ"))


if __name__ == "__main__":
    unittest.main()
