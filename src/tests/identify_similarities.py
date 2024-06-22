from difflib import SequenceMatcher
# יבוא פונקציה לקריאת עץ תיקיות
from os.path import join, getsize

def find_text_similarity(text, text_list):
    """
בדיקת דמיון בין מחרוזת מסויימת לרשימת מחרוזות
    
פרמטרים:
    פרמטר 1 = מחרוזת טקסט
    פרמטר 2 = רשימת מחרוזות טקסט

תוצאה:
    טאפל עם 3 משתנים:
    אייטם 1 - שם השיר הדומה ביותר
    אייטם 2 - מספר המייצג את רמת הדמיון
    אייטם 3 - כרגע מחזיר "None"
    """
    # יצירת רשימת דמיונים
    similarity_list = []
    for item in text_list:
        # יצירת אובייקט שמשמש לזיהוי דמיון בין המחרוזות
        sequence_matcher = SequenceMatcher(None, text, item)
        # חישוב אחוז הדמיון בין המחרוזות
        similarity = sequence_matcher.ratio()
        # הוספת הדמיון לרשימת הדמיונים
        similarity_list.append(similarity)

    # יצירת רשימה עם המחרוזות ורמת הדמיון שלהם
    num = 0
    sum_list = []
    for item in text_list:
        name_num = (similarity_list[num], item)
        sum_list.append(name_num)
        num += 1
    sum_list.sort(reverse=True)
        
    # מציאת הדמיון המקסימלי ודילוג על זיהוי מלא
    max_similarity = max(similarity_list)
    if  max_similarity == 1.0:
        max_similarity = sorted(set(similarity_list))[-2]
    
    # מציאת המחרוזת הדומה ביותר ודילוג על מחרוזת זהה לחלוטין
    most_similar_string = text_list[similarity_list.index(max_similarity)]

    return most_similar_string, max_similarity, None


# הפונקציה מקבלת ערכי מספרים מפונקציית "find_text_similarity"
# הפונקציה מחזירה אמת אם המחרוזות מתאימות
def similarity_sure(text, text_list, Similarity_sure=True):
    """
מחשב את רמת ההתאמה בין מחרוזות, ומחזיר אמת או שקר
בהתאם לאורך המחרוזת לחיפוש
הפונקציה מחזירה אמת רק אם רמת ההתאמה גבוהה מאוד

פרמטרים:
    פרמטר 1 = מחרוזת טקסט
    פרמטר 2 = רשימת מחרוזות טקסט
    פרמטר 3 = אופציונלי - הגדרת התאמה גבוהה או בינונית.
ניתן להכניס אמת או שקר. ברירת המחדל היא אמת.
    
תוצאה:
    "True" + שם המחרוזת הדומה ביותר' או "False" + "None"
    """
    # הפעלת הפונקציה לזיהוי דמיון בין מחרוזות וקבלת ערכי משתנים
    most_similar_string, max_similarity, sum_list = find_text_similarity(text, text_list)
    
    # ניתוח ערכי המשתנים בהתאם לאורך המחרוזת
    str_len = len(text.replace(" ", ""))

    # הגדרת רמת ההתאמה הנדרשת בתרגיל חשבוני בהתאם לפרמטר שהוכנס לפונקציה
    if Similarity_sure == False:
        Required_level_similarity = 1.0 - (str_len * 0.02)
    elif Similarity_sure == True:
        Required_level_similarity = 1.0 - (str_len * 0.01)
    
    # אם רמת הדמיון מספקת, החזר אמת, אם לא החזר שקר
    if max_similarity >= Required_level_similarity:
        return True, most_similar_string
    else:
        return False, None


def main():
    import os
    text_list = os.listdir()
    for text in text_list:
        most_similar_string, max_similarity, sum_list = find_text_similarity(text, text_list)
        print(text)
        print(f'השם הדומה ביותר למחרוזת הראשונה הוא "{most_similar_string}" עם דמיון של {max_similarity:.2f}')
        print("-" * 70)
        
        # הדפסת רשימת המחרוזות עם רמת הדמיון שלהם למחרוזת הנוכחית
    if False:
        num = 0
        for i in sum_list:
            print("{} {} {}".format(i[0], "=", i[1]))
            if num == 1:
                break
            num += 1

if __name__ == '__main__':
    main()

