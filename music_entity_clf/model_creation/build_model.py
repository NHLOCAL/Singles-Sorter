import csv
import pickle
from sklearn.model_selection import train_test_split, GridSearchCV
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.linear_model import LogisticRegression
from sklearn.pipeline import make_pipeline
from sklearn import metrics
import matplotlib.pyplot as plt
import seaborn as sns

# מיפוי תוויות מספריות לשמות קטגוריות
label_mapping = {0: "ARTIST", 1: "ALBUM", 2: "SONG", 3: "RANDOM"}

# רשימות ריקות לטעינת הדאטה
texts = []
labels = []

# קריאת הדאטה מקובץ CSV
with open('dataset.csv', newline='', encoding='utf-8') as csvfile:
    reader = csv.DictReader(csvfile)
    
    for row in reader:
        try:
            texts.append(row['text'])
            label_value = row['label']
            if label_value.isdigit():  # בדיקה אם הערך הוא מספר שלם
                labels.append(int(label_value))
            else:
                print(f"ערך לא תקין בתווית: {label_value}, דילוג על שורה")
        except Exception as e:
            print(f"שגיאה בקריאת שורה: {e}, דילוג על שורה")

# חלוקת הדאטה לסט אימון וסט בדיקה
X_train, X_test, y_train, y_test = train_test_split(texts, labels, test_size=0.2, random_state=42)

# יצירת פייפליין שממיר את הטקסטים למטריצת TF-IDF ואז מאמן מודל Logistic Regression
model = make_pipeline(TfidfVectorizer(), LogisticRegression())

# הגדרת רשת פרמטרים לבדיקה
param_grid = {
    'logisticregression__max_iter': [150, 200],
    'tfidfvectorizer__ngram_range': [(1, 2)],
}

# יצירת אובייקט GridSearchCV
grid_search = GridSearchCV(model, param_grid, cv=5)

# אימון המודל עם חיפוש רשת
grid_search.fit(X_train, y_train)

# הדפסת הפרמטרים הטובים ביותר שנמצאו
print(f'הפרמטרים הטובים ביותר: {grid_search.best_params_}')

# שימוש במודל הטוב ביותר שנמצא
best_model = grid_search.best_estimator_

# בדיקת המודל הטוב ביותר
accuracy = best_model.score(X_test, y_test)
print(f'דיוק המודל הטוב ביותר: {accuracy * 100:.2f}%')

# שמירת המודל
with open('music_classifier.pkl', 'wb') as f:
    pickle.dump(best_model, f)

# טעינת המודל (לא חובה, רק להדגמה)
with open('music_classifier.pkl', 'rb') as f:
    loaded_model = pickle.load(f)

# דוגמה לשימוש במודל לחיזוי על טקסט חדש
new_text = "שלום חנוך"
predicted_label = loaded_model.predict([new_text])[0]
print(f'טקסט: "{new_text}", תווית חזוי: {label_mapping[predicted_label]}')

# יצירת Confusion Matrix
predicted = loaded_model.predict(X_test)
cm = metrics.confusion_matrix(y_test, predicted)

# ויזואליזציה של Confusion Matrix
plt.figure(figsize=(10, 7))
sns.heatmap(cm, annot=True, fmt='d', cmap='Blues', 
            xticklabels=label_mapping.values(), yticklabels=label_mapping.values())
plt.xlabel('Predicted Label')
plt.ylabel('True Label')
plt.title('Confusion Matrix')
plt.show()

# הדפסת דוח סיווג
print(metrics.classification_report(y_test, predicted, target_names=label_mapping.values()))