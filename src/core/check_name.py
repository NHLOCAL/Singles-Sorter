# -*- coding: utf-8 -*-
import os
import re
import csv

WORD_PATTERN = re.compile(r"[א-תA-Za-z0-9'\"׳]+")


def artist_from_song(my_file):
    """
    הפונקציה בודקת את שם האמן בשם הקובץ על סמך מסד נתונים ומאחסנת את שם האמן במשתנה.
        אם השם לא קיים, הוא סורק את המטא נתונים של השיר ומאחסן את שם האמן במשתנה.

        פרמטר:
            my_file (str): שם הקובץ שיש לסרוק.

        החזרות:
            str: הערך המכיל את שם האמן מהקובץ.
    """

    # קבל את שם הקובץ ללא הנתיב המלא
    split_file = os.path.split(my_file)[1]
    split_file = os.path.splitext(split_file)[0]

    # הסר תווים לא רצויים משם הקובץ
    split_file = re.sub(r'[_-]', ' ', split_file)

    # ייבא את רשימת הזמרים מקובץ csv
    if 'singer_list' not in globals():
        csv_path = "singer-list.csv"
        global singer_list
        with open(csv_path, 'r') as file:
            csv_reader = csv.reader(file)
            singer_list = [tuple(row) for row in csv_reader]

        if os.path.isfile("personal-singer-list.csv"):
            with open("personal-singer-list.csv", 'r') as file:
                csv_reader = csv.reader(file)
                personal_list = [tuple(row) for row in csv_reader]
            singer_list.extend(personal_list)

    # חזור על רשימת השמות ובדוק אם אחד מהם קיים בשם הקובץ
    for source_name, target_name in singer_list:
        # בדיקת דיוק/דמיון שם הקובץ
        if check_exact_name(split_file, source_name):
            return target_name

    return


def check_exact_name(filename, artist_to_search):
    """
    Check if the artist's name appears exactly in the filename, even if preceded by "ו".

    Parameters:
    filename (str): The filename or metadata.
    artist_to_search (str): The artist's name to search for.

    Returns:
    bool: True if the artist's name is found exactly in the filename (even if preceded by "ו"), False otherwise.
    """

    if not filename or not artist_to_search:
        return False

    # Remove leading spaces in the filename
    filename = filename.lstrip()

    # Escape special characters in the artist's name
    escaped_artist = re.escape(artist_to_search)

    # Define a pattern to match the exact artist name, even if preceded by "ו"
    exact_match_pattern = fr'(^|[^א-ת])ו?{escaped_artist}\b'

    # Search for the exact artist name in the filename
    if re.search(exact_match_pattern, filename):
        return True

    return _is_similar_name_match(filename, artist_to_search)


def _tokenize_words(text):
    return WORD_PATTERN.findall(text)


def _max_allowed_word_distance(word_length):
    if word_length <= 4:
        return 0
    if word_length <= 8:
        return 1
    if word_length <= 12:
        return 2
    return 3


def _max_allowed_phrase_distance(total_letters):
    if total_letters <= 8:
        return 1
    if total_letters <= 15:
        return 2
    if total_letters <= 24:
        return 3
    return 4


def _is_prefix_or_suffix_expansion(first_word, second_word):
    if abs(len(first_word) - len(second_word)) != 1:
        return False

    longer_word, shorter_word = (
        (first_word, second_word)
        if len(first_word) > len(second_word)
        else (second_word, first_word)
    )

    # מניעת false positive כמו "אלי" מול "יואלי" או "מוטי" מול "למוטי".
    return longer_word[1:] == shorter_word or longer_word[:-1] == shorter_word


def _levenshtein_distance(first_word, second_word, max_distance=None):
    if first_word == second_word:
        return 0

    if len(first_word) < len(second_word):
        first_word, second_word = second_word, first_word

    previous_row = list(range(len(second_word) + 1))

    for index_first, char_first in enumerate(first_word, start=1):
        current_row = [index_first]
        min_in_row = current_row[0]

        for index_second, char_second in enumerate(second_word, start=1):
            insertions = previous_row[index_second] + 1
            deletions = current_row[index_second - 1] + 1
            substitutions = previous_row[index_second - 1] + (char_first != char_second)
            best_cost = min(insertions, deletions, substitutions)
            current_row.append(best_cost)
            if best_cost < min_in_row:
                min_in_row = best_cost

        if max_distance is not None and min_in_row > max_distance:
            return max_distance + 1

        previous_row = current_row

    return previous_row[-1]


def _is_similar_name_match(filename, artist_to_search):
    filename_words = _tokenize_words(filename)
    artist_words = _tokenize_words(artist_to_search)

    if not filename_words or not artist_words:
        return False

    artist_words_count = len(artist_words)
    max_window_index = len(filename_words) - artist_words_count + 1

    if max_window_index <= 0:
        return False

    total_artist_letters = sum(len(word) for word in artist_words)
    max_phrase_distance = _max_allowed_phrase_distance(total_artist_letters)

    for start_index in range(max_window_index):
        candidate_words = filename_words[start_index:start_index + artist_words_count]
        total_distance = 0

        for candidate_word, artist_word in zip(candidate_words, artist_words):
            if _is_prefix_or_suffix_expansion(candidate_word, artist_word):
                break

            max_word_distance = _max_allowed_word_distance(len(artist_word))
            word_distance = _levenshtein_distance(candidate_word, artist_word, max_word_distance)

            if word_distance > max_word_distance:
                break

            total_distance += word_distance
            if total_distance > max_phrase_distance:
                break
        else:
            return True

    return False


if __name__ == '__main__':

    list_ = ['ח בני פרידמן, מוטי שטיינמ.mp3', '@יואלי קליין=.mp3', 'ואברהם פריד.mp3', 'שיר נוסף - מוטי שטיינמץל מ.mp3']

    for i in list_:
        print(artist_from_song(i))
