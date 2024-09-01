import pickle
import csv
from sklearn import metrics
import matplotlib.pyplot as plt
import seaborn as sns

# מיפוי תוויות מספריות לשמות קטגוריות
label_mapping = {0: "ARTIST", 1: "ALBUM", 2: "SONG", 3: "RANDOM"}

# טעינת המודל
with open('music_classifier.pkl', 'rb') as f:
    loaded_model = pickle.load(f)

# רשימות ריקות לטעינת הדאטה
texts = []
labels = []

# קריאת הדאטה מקובץ CSV
with open('test_set.csv', newline='', encoding='utf-8') as csvfile:
    reader = csv.DictReader(csvfile)
    
    for row in reader:
        try:
            texts.append(row['text'])
            labels.append(int(row['label']))  # המרת תוויות למספרים שלמים
        except Exception as e:
            print(f"שגיאה בקריאת שורה: {e}, דילוג על שורה")

# חיזוי על כל הדאטה
predicted = loaded_model.predict(texts)

# יצירת Confusion Matrix
cm = metrics.confusion_matrix(labels, predicted)

# ויזואליזציה של Confusion Matrix
plt.figure(figsize=(10, 7))
sns.heatmap(cm, annot=True, fmt='d', cmap='Blues', 
            xticklabels=label_mapping.values(), yticklabels=label_mapping.values())
plt.xlabel('Predicted Label')
plt.ylabel('True Label')
plt.title('Confusion Matrix')
plt.show()

# הדפסת דוח סיווג
print(metrics.classification_report(labels, predicted, target_names=label_mapping.values()))

# חישוב דיוק כללי
accuracy = metrics.accuracy_score(labels, predicted)
print(f'דיוק כללי: {accuracy * 100:.2f}%')