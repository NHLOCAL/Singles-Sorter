import matplotlib.pyplot as plt
import json

# לקרוא את שם המודל
with open("model_name.txt", 'r', encoding='utf-8') as f:
    model_name = f.read()
    print(f'# {model_name}')

# לקרוא את נתוני האיטרציה מקובץ ה-JSON
with open(f'{model_name}\\iteration_data.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

# חילוץ ערכי ציר ה-Y
y_values = [data[str(key)]['ner'] for key in sorted(map(int, data.keys()))]

# יצירת הגרף
plt.figure(figsize=(10, 6))
plt.plot(range(len(data)), y_values, marker='o', linestyle='-', color='b')

# הוספת תוויות וכותרת
plt.xlabel('Iteration')
plt.ylabel('NER Loss')
plt.title('NER Loss per Iteration')

# הצגת קווי רשת אופקיים נוספים בצפיפות כפולה
plt.minorticks_on()

# הגדרת קווי רשת אופקיים
plt.gca().yaxis.set_major_locator(plt.MultipleLocator(1000))
plt.gca().yaxis.set_minor_locator(plt.MultipleLocator(250))

# הגדרת קווי רשת לאורך
plt.gca().xaxis.set_major_locator(plt.MultipleLocator(10))

# הצגת קווי רשת
plt.grid(which='major', linestyle='-', linewidth=1, color='black')  # קווים מודגשים
plt.grid(which='minor', linestyle='--', linewidth=0.4, color='gray')  # קווים פחות מודגשים

# הצגת הגרף
#plt.savefig('ner_loss_graph.png')
plt.show()
