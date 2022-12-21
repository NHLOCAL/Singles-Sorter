from difflib import SequenceMatcher

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
    אייטם 3 - רשימה של כל הפריטים עם רמת הדמיון שלהם לפריט היחיד
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
        
  
    # מציאת הדמיון המקסימלי
    max_similarity = max(similarity_list)
  
    # מציאת המחרוזת הדומה ביותר
    most_similar_string = text_list[similarity_list.index(max_similarity)]

    return most_similar_string, max_similarity, sum_list


# הפונקציה מקבלת ערכי מספרים מפונקציית "find_text_similarity"
# הפונקציה מחזירה אמת אם המחרוזות מתאימות
def Similarity_sure(text, text_list, Similarity_sure=True):
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
    אמת או שקר
    """
    # הפעלת הפונקציה לזיהוי דמיון בין מחרוזות וקבלת ערכי משתנים
    most_similar_string, max_similarity, sum_list = find_text_similarity(text, text_list)
    
    # ניתוח ערכי המשתנים בהתאם לאורך המחרוזת
    str_len = len(text.replace(" ", ""))

    # הגדרת רמת ההתאמה הנדרשת בתרגיל חשבוני בהתאם לפרמטר שהוכנס לפונקציה
    if Similarity_sure == False:
        Required_level_similarity = 1.0 - (str_len * 0.02)
    elif Similarity_sure == True:
        Required_level_similarity = 1.0 - (str_len * 0.015)
    
    # אם רמת הדמיון מספקת, החזר אמת, אם לא החזר שקר
    if max_similarity >= Required_level_similarity:
        return True
    else:
        return False


def main():
    import os
    text_list = os.listdir(r"J:\שמע\מוזיקה נוספת\יעד")
    for text in text_list:
           
        most_similar_string, max_similarity, sum_list = find_text_similarity(text, text_list)
        print(f'השם הדומה ביותר למחרוזת הראשונה הוא "{most_similar_string}" עם דמיון של {max_similarity:.2f}')
        print("-" * 70)
        
        # הדפסת רשימת המחרוזות עם רמת הדמיון שלהם למחרוזת הנוכחית
        for i in sum_list:
            print("{} {} {}".format(i[0], "=", i[1]))


if __name__ == '__main__':
    main()

