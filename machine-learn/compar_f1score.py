import spacy
from spacy.training.example import Example
from tabulate import tabulate
import os
import json

# Function to evaluate model and return evaluation metrics
def evaluate_model(model_name, data):
    nlp = spacy.load(model_name)

    filtered_data = [
        (text, {'entities': [(start, end, label) for start, end, label in annots['entities'] if label == 'SINGER']})
        for text, annots in data
    ]

    # Calculate metrics manually
    true_positive = 0
    false_positive = 0
    false_negative = 0

    for text, annotations in filtered_data:
        doc = nlp(text)
        pred_entities = [(ent.start_char, ent.end_char, ent.label_) for ent in doc.ents if ent.label_ == 'SINGER']
        true_entities = [(start, end, label) for start, end, label in annotations['entities']]

        # Calculate True Positives
        for entity in pred_entities:
            if entity in true_entities:
                true_positive += 1
            else:
                false_positive += 1

        # Calculate False Negatives
        for entity in true_entities:
            if entity not in pred_entities:
                false_negative += 1

    # Precision, Recall, and F1 Score calculations
    precision = true_positive / (true_positive + false_positive) if (true_positive + false_positive) > 0 else 0
    recall = true_positive / (true_positive + false_negative) if (true_positive + false_negative) > 0 else 0
    f1_score = (2 * precision * recall) / (precision + recall) if (precision + recall) > 0 else 0

    return f1_score, precision, recall


# Load data from data.json
with open("f1score_dataset_v2.json", 'r', encoding='utf-8') as f:
    data = json.load(f)

# Create a list to store model names and their metrics
models_metrics = []

# Iterate over model versions
model_names = [f"models/{i}" for i in os.listdir("models") if "custom_ner_model" in i]


for model_name in model_names:
    print(f"Evaluating {model_name}...")
    f1_score, precision, recall = evaluate_model(model_name, data)
    models_metrics.append({
        "Model": model_name,
        "F1 Score": f"{f1_score:.6f}",
        "Precision": f"{precision:.6f}",
        "Recall": f"{recall:.6f}"
    })

# Sort by F1 Score
models_metrics_sorted = sorted(models_metrics, key=lambda x: x["F1 Score"], reverse=True)

# Prepare data for tabulate
table = [["Model", "F1 Score", "Precision", "Recall"]]
for entry in models_metrics_sorted:
    table.append([entry["Model"], entry["F1 Score"], entry["Precision"], entry["Recall"]])

# Print the table
print("\nModel Evaluation Results:")
print(tabulate(table, headers="firstrow", tablefmt="grid"))

# Optional: Save results to a CSV file
import pandas as pd
df = pd.DataFrame(models_metrics_sorted)
df.to_csv("model_metrics.csv", index=False)
