import unittest
from singles_sorter_v5 import MusicSorter

class TestFilenameCleanup(unittest.TestCase):
    def setUp(self):
        self.cleaner = MusicSorter("c:", "c:")

    def test_basic_underscores(self):
        """בדיקת החלפת קווים תחתונים בסיסית"""
        input_name = "בנצי_שטיין_טאטאלע"
        expected = "בנצי שטיין טאטאלע"
        self.assertEqual(self.cleaner.clean_filename(input_name), expected)
        
    def test_subsrings_remove(self):
        """בדיקת הסרת מחרוזות מיוחדות"""
        input_name = "מייל מיוזיק - שיר חדש נשיר"
        expected = "שיר חדש נשיר"
        self.assertEqual(self.cleaner.clean_filename(input_name), expected)

    def test_multiple_consecutive_underscores(self):
        """בדיקת רצף קווים תחתונים"""
        input_name = "בנצי___שטיין___טאטאלע"
        expected = "בנצי שטיין טאטאלע"
        self.assertEqual(self.cleaner.clean_filename(input_name), expected)

    def test_underscore_within_word(self):
        """בדיקת קו תחתון בתוך מילה"""
        input_name = "להקת הנח_לים שרה"
        expected = "להקת הנחלים שרה"
        self.assertEqual(self.cleaner.clean_filename(input_name), expected)

    def test_mixed_cases(self):
        """בדיקת מקרים מעורבים"""
        test_cases = [
            ("להקת הנח_לים שרה", "להקת הנחלים שרה"),
            ("שיר המע_לות", "שיר המעלות"),
            ("בנצי_שטיין___טאטאלע___קולולחן___ווקאלי", "בנצי שטיין טאטאלע קולולחן ווקאלי")
        ]
        for input_name, expected in test_cases:
            with self.subTest(input_name=input_name):
                self.assertEqual(self.cleaner.clean_filename(input_name), expected)

    def test_edge_cases(self):
        """בדיקת מקרי קצה"""
        test_cases = [
            ("", ""),
            ("מילה", "מילה"),
            ("__בנצי__שטיין__", "בנצי שטיין"),
            ("בנצי-שטיין", "בנצי שטיין"),
            ("בנצי   שטיין", "בנצי שטיין")
        ]
        for input_name, expected in test_cases:
            with self.subTest(input_name=input_name):
                self.assertEqual(self.cleaner.clean_filename(input_name), expected)

    def test_substrings_removal(self):
        """בדיקת הסרת תתי-מחרוזות"""
        # יש להגדיר SUBSTRINGS_TO_REMOVE = ["TEST_"]
        input_name = "בנצי שטיין המחדש"
        expected = "בנצי שטיין"
        self.assertEqual(self.cleaner.clean_filename(input_name), expected)
    
    def test_mixed_separators(self):
        """בדיקת שילוב של מפרידים שונים"""
        input_name = "בנצי-שטיין_טאטאלע.ווקאלי"
        expected = "בנצי שטיין טאטאלע ווקאלי"
        self.assertEqual(self.cleaner.clean_filename(input_name), expected)

    def test_hebrew_with_english(self):
        """בדיקת שילוב עברית ואנגלית"""
        input_name = "בנצי_Stein_טאטאלע"
        expected = "בנצי Stein טאטאלע"
        self.assertEqual(self.cleaner.clean_filename(input_name), expected)

    def test_special_cases(self):
        """בדיקת מקרים מיוחדים"""
        input_name = "בנצי-שטיין-טאטא"
        expected = "בנצי שטיין טאטא"
        self.assertEqual(self.cleaner.clean_filename(input_name), expected)

    def test_inside_word_underscore(self):
        """בדיקת קו תחתון בתוך מילה עם מקרים נוספים"""
        test_cases = [
            ("המע_לות_שיר", "המע לות שיר"),
            ("שיר המע_לות", "שיר המעלות"),
            ("להקת הנח_לים שרה", "להקת הנחלים שרה")
        ]
        for input_name, expected in test_cases:
            with self.subTest(input_name=input_name):
                self.assertEqual(self.cleaner.clean_filename(input_name), expected)

if __name__ == '__main__':
    unittest.main()