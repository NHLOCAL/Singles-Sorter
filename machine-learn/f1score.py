import sys
import spacy
from spacy.training.example import Example
import json
from collections import defaultdict
from tabulate import tabulate

# read name of model
if len(sys.argv) > 1:
    model_name = f"custom_ner_model{sys.argv[1]}git"
    print(f"# {model_name}")
else:
    with open("model_name.txt", 'r', encoding='utf-8') as f:
        model_name = f.read()
        print(f'# {model_name}')

# Load your trained model
nlp = spacy.load(f"models/{model_name}")

def find_overlapping_entities(data):
    overlapping_entries = []
    for i, (text, annots) in enumerate(data):
        entities = annots.get('entities', [])
        for j in range(len(entities)):
            for k in range(j + 1, len(entities)):
                start_j, end_j, label_j = entities[j]
                start_k, end_k, label_k = entities[k]

                # Check for overlap
                if not (end_j <= start_k or end_k <= start_j):
                    overlapping_entries.append({
                        "index": i,
                        "text": text,
                        "entity1": (start_j, end_j, label_j),
                        "entity2": (start_k, end_k, label_k)
                    })
    return overlapping_entries

# Load data from data.json
with open("f1score_dataset_v2.json", 'r', encoding='utf-8') as f:
    data = json.load(f)

overlapping = find_overlapping_entities(data)

if overlapping:
    print("Found overlapping entities in the following entries:")
    for entry in overlapping:
        print(f"Index: {entry['index']}")
        print(f"Text: {entry['text']}")
        print(f"Entity 1: {entry['entity1']}")
        print(f"Entity 2: {entry['entity2']}")
        print("-" * 20)
    sys.exit(1)  # Exit if overlapping entities are found

examples = []
for text, annots in data:
    doc = nlp.make_doc(text)
    gold_dict = {'entities': annots.get('entities', [])}
    example = Example.from_dict(doc, gold_dict)
    examples.append(example)

all_labels = nlp.pipe_labels['ner']

entity_metrics = {}
overall_tp = 0
overall_fp = 0
overall_fn = 0

for label in all_labels:
    true_positive = 0
    false_positive = 0
    false_negative = 0

    for text, annotations in data:
        doc = nlp(text)
        pred_entities = [(ent.start_char, ent.end_char, ent.label_) for ent in doc.ents if ent.label_ == label]
        true_entities = [(start, end, l) for start, end, l in annotations.get('entities', []) if l == label]

        for entity in pred_entities:
            if entity in true_entities:
                true_positive += 1
            else:
                false_positive += 1

        for entity in true_entities:
            if entity not in pred_entities:
                false_negative += 1

    overall_tp += true_positive
    overall_fp += false_positive
    overall_fn += false_negative

    precision = true_positive / (true_positive + false_positive) if (true_positive + false_positive) > 0 else 0
    recall = true_positive / (true_positive + false_negative) if (true_positive + false_negative) > 0 else 0
    f1_score = (2 * precision * recall) / (precision + recall) if (precision + recall) > 0 else 0

    entity_metrics[label] = {"precision": precision, "recall": recall, "f1_score": f1_score}

def analyze_errors(nlp, data, labels):
    errors = []
    for label in labels:
        for text, true_entities in data:
            true_entities_filtered = [
                (start, end, l) for start, end, l in true_entities.get('entities', []) if l == label
            ]
            doc = nlp(text)
            pred_entities_filtered = [
                (ent.start_char, ent.end_char, ent.label_) for ent in doc.ents if ent.label_ == label
            ]

            for start, end, l in pred_entities_filtered:
                if (start, end, l) not in true_entities_filtered:
                    errors.append({
                        "text": text,
                        "error_type": "False Positive",
                        "entity": text[start:end],
                        "start": start,
                        "end": end,
                        "label": l
                    })

            for start, end, l in true_entities_filtered:
                if (start, end, l) not in pred_entities_filtered:
                    errors.append({
                        "text": text,
                        "error_type": "False Negative",
                        "entity": text[start:end],
                        "start": start,
                        "end": end,
                        "label": l
                    })
    return errors

errors = analyze_errors(nlp, data, all_labels)

print("\n--- Incorrect Identifications ---")
for error in errors:
    print(f"Text: {error['text']}")
    print(f"Error Type: {error['error_type']}")
    print(f"Entity: {error['entity']} ({error['label']})")
    print(f"Position: {error['start']}:{error['end']}")
    print()

print(f"Total errors: {len(errors)}")

# Calculate overall metrics for the summary table
overall_precision = overall_tp / (overall_tp + overall_fp) if (overall_tp + overall_fp) > 0 else 0
overall_recall = overall_tp / (overall_tp + overall_fn) if (overall_tp + overall_fn) > 0 else 0
overall_f1_score = (2 * overall_precision * overall_recall) / (overall_precision + overall_recall) if (overall_precision + overall_recall) > 0 else 0

# Prepare data for tabulate
table_data = []
for label, metrics in entity_metrics.items():
    table_data.append([label, f"{metrics['precision']:.4f}", f"{metrics['recall']:.4f}", f"{metrics['f1_score']:.4f}"])

# Add overall metrics to the table
table_data.append(['Overall', f"{overall_precision:.4f}", f"{overall_recall:.4f}", f"{overall_f1_score:.4f}"])

# Print the table
print("\n--- Evaluation Summary ---")
headers = ["Entity Type", "Precision", "Recall", "F1 Score"]
print(tabulate(table_data, headers=headers, tablefmt="grid"))