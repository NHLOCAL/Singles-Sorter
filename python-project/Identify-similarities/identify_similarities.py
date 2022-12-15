from difflib import SequenceMatcher

def find_text_similarity(text, text_list):
    """
    בדיקת דמיון בין מחרוזת מסויימת לרשימת מחרוזות
    
    פרמטרים:
    פרמטר 1 = מחרוזת טקסט
    פרמטר 2 = רשימת מחרוזות טקסט
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

def main():
    text = "אברימי רוט"
    text_list = ["אבריימי רוט", "אברימי לוי", "אברהם פריד", "מוטי שטיינמץ", "אברמי רוט", "אברימי רוט"]
    most_similar_string, max_similarity,  sum_list = find_text_similarity(text, text_list)
    print(f'השם הדומה ביותר למחרוזת הראשונה הוא "{most_similar_string}" עם דמיון של {max_similarity:.2f}')
    print("-" * 70)
    for i in sum_list:
        print("{} {} {}".format(i[0], "=", i[1]))


if __name__ == '__main__':
    main()

