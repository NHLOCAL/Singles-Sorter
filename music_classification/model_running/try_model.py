import pickle

# מיפוי תוויות מספריות לשמות קטגוריות
label_mapping = {0: "ARTIST", 1: "ALBUM", 2: "SONG", 3: "RANDOM"}

# טעינת המודל
with open('music_classifier.pkl', 'rb') as f:
    loaded_model = pickle.load(f)

# פונקציה לחיזוי על מחרוזת טקסט בודדת
def classify_text(text):
    # המודל מצפה לקבל רשימה של טקסטים גם אם יש טקסט אחד בלבד
    prediction = loaded_model.predict([text])
    return prediction[0]  # החיזוי הוא רשימה ולכן אנו מחזירים את הערך הראשון

# הדגמת השימוש בפונקציה
text_input = input("הכנס מחרוזת טקסט לסיווג: ")
prediction = classify_text(text_input)

# הצגת התוצאה
print(f"הקטגוריה של הטקסט היא: {label_mapping[prediction]} (זיהוי מספרי: {prediction})")
