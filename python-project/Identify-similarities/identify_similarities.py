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
    for text2 in text_list:
        # יצירת אובייקט שמשמש לזיהוי דמיון בין המחרוזות
        sequence_matcher = SequenceMatcher(None, text, text2)
        # חישוב אחוז הדמיון בין המחרוזות
        similarity = sequence_matcher.ratio()
        # הוספת הדמיון לרשימת הדמיונים
        similarity_list.append(similarity)
  
    # מציאת הדמיון המקסימלי
    max_similarity = max(similarity_list)
  
    # מציאת המחרוזת הדומה ביותר
    most_similar_string = text_list[similarity_list.index(max_similarity)]
  
    return (most_similar_string, max_similarity)




def main(text, text_list):
    text = "אברימי רוט"
    text_list = ["אבריימי רוט", "אברימי לוי", "אברהם פריד", "מוטי שטיינמץ"]
    most_similar_string, max_similarity = find_text_similarity(text, text_list)
    print(f'השם הדומה ביותר למחרוזת הראשונה הוא "{most_similar_string}" עם דמיון של {max_similarity:.2f}')



if __name__ == '__main__':
    main()

