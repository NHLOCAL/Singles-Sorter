def is_similar_name(name1, name2):
    """
    בודק אם שני שמות דומים בהבדל של אות אחת בלבד.
    """
    # בדיקה מהירה עבור התאמה מדויקת
    if name1 == name2:
        return True

    # בדיקה עבור הבדל של אות אחת
    differences = 0
    for a, b in zip(name1, name2):
        if a != b:
            differences += 1
            if differences > 1:
                return False
    return differences == 1


# דוגמאות לשימוש
print(is_similar_name("וייסמנדל", "ויסמנדל"))  # True
print(is_similar_name("אלי קליין", "יואלי קליין"))  # False
print(is_similar_name("אברהם פריד", "אברהם פרידמן"))  # False
print(is_similar_name("מרדכי בן דוד", "מרדכי בן דויד"))  # True