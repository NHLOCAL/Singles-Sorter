import sys
import spacy
from spacy.training.example import Example
import json

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


examples = []

# Load data from data.json
with open("f1score_dataset_v1.json", 'r', encoding='utf-8') as f:
    data = json.load(f)


for text, annots in data:
    doc = nlp.make_doc(text)
    examples.append(Example.from_dict(doc, annots))
print(nlp.evaluate(examples))



def analyze_errors(nlp, data):
    errors = []
    for text, true_entities in data:
        # Filter entities to include only 'SINGER'
        true_entities_filtered = [
            (start, end, label) for start, end, label in true_entities['entities'] if label == 'SINGER'
        ]
        doc = nlp(text)
        pred_entities_filtered = [
            (ent.start_char, ent.end_char, ent.label_) for ent in doc.ents if ent.label_ == 'SINGER'
        ]

        # Check for false positives and false negatives
        for start, end, label in set(pred_entities_filtered + true_entities_filtered):
            pred = (start, end, label) in pred_entities_filtered
            true = (start, end, label) in true_entities_filtered

            if pred != true:
                error_type = "False Positive" if pred else "False Negative"
                errors.append({
                    "text": text,
                    "error_type": error_type,
                    "entity": text[start:end],
                    "start": start,
                    "end": end,
                    "label": label
                })

    return errors


errors = analyze_errors(nlp, data)

print("\nIncorrect Identifications:")
for error in errors:
    print(f"Text: {error['text']}")
    print(f"Error Type: {error['error_type']}")
    print(f"Entity: {error['entity']}")
    print(f"Position: {error['start']}:{error['end']}")
    #print(f"Label: {error['label']}")
    print()

print(f"Total errors: {len(errors)}")