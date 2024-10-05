import spacy
from spacy.lang.he import Hebrew
from spacy.tokenizer import Tokenizer
from spacy.training.example import Example
from spacy.lang.char_classes import LIST_PUNCT, LIST_ELLIPSES, LIST_QUOTES, LIST_CURRENCY, LIST_ICONS
from spacy.util import minibatch, compounding
import json
import random
import os
import tqdm
import numpy as np


def custom_tokenizer(nlp):
    default_tokenizer = Tokenizer(nlp.vocab)
    nlp2 = Hebrew()

    LIST_BREAKING_WORDS = [r'—', r'--', r'-', r'\+']
    LIST_SYMBOLS = [r"[\x2D&]", r"”", "\."]

    custom_patterns = (
        LIST_QUOTES +
        LIST_ELLIPSES +
        LIST_BREAKING_WORDS +
        LIST_SYMBOLS +
        LIST_CURRENCY +
        LIST_PUNCT +
        LIST_ICONS
    )

    prefixes = nlp2.Defaults.prefixes + custom_patterns + [r'^(?!וו)ו']
    infixes = nlp2.Defaults.infixes + custom_patterns
    suffixes = nlp2.Defaults.suffixes + custom_patterns

    prefix_regex = spacy.util.compile_prefix_regex(prefixes)
    infix_regex = spacy.util.compile_infix_regex(infixes)
    suffix_regex = spacy.util.compile_suffix_regex(suffixes)

    nlp2.tokenizer.prefix_search = prefix_regex.search
    nlp2.tokenizer.infix_finditer = infix_regex.finditer
    nlp2.tokenizer.suffix_search = suffix_regex.search

    return nlp2.tokenizer


nlp = spacy.blank("he")
nlp.tokenizer = custom_tokenizer(nlp)

# Adding NER pipe
if "ner" not in nlp.pipe_names:
    ner = nlp.add_pipe("ner")
ner.add_label("SINGER")

json_files = [
    os.path.join(os.getenv('GITHUB_WORKSPACE', '.'), 'machine-learn', 'scrape_data', 'cleaned_new-data.json')
]

training_data = []
for json_file in json_files:
    if os.path.exists(json_file):
        with open(json_file, 'r', encoding='utf-8') as f:
            data = json.load(f)
            for example_text, example_entities in data:
                entities = example_entities.get('entities', [])
                example = Example.from_dict(nlp.make_doc(example_text), {'entities': entities})
                training_data.append(example)
    else:
        print(f"Warning: {json_file} not found.")

# Begin training
optimizer = nlp.create_optimizer()
patience = 10
min_delta = 0.001
best_loss = float('inf')
patience_counter = 0
best_model_path = os.path.join(os.getenv('GITHUB_WORKSPACE', '.'), "machine-learn", "best_model")
n_iter = 100
batch_size = 32
drop_size = 0.3
iteration_data = {}
learning_rates = np.linspace(0.001, 0.0001, n_iter)

for itn in tqdm.tqdm(range(n_iter), desc="Training iterations"):
    random.shuffle(training_data)
    losses = {}
    batches = minibatch(training_data, size=batch_size)
    for batch in batches:
        nlp.update(batch, drop=drop_size, sgd=optimizer, losses=losses)
    print(f"Iteration {itn}: {losses}")
    iteration_data[itn] = losses.copy()

    current_loss = losses.get('ner', float('inf'))
    if current_loss < best_loss - min_delta:
        best_loss = current_loss
        patience_counter = 0
        # Save the best model
        nlp.to_disk(best_model_path)
    else:
        patience_counter += 1

    # Update learning rate
    for g in optimizer.optimizer.param_groups:
        g['lr'] = learning_rates[itn]

    if patience_counter >= patience:
        print(f"Early stopping at iteration {itn}")
        break

# Load the best model before saving with the final name
if os.path.exists(best_model_path):
    nlp = spacy.load(best_model_path)
    nlp.meta['name'] = 'singer_ner_he'
    nlp.meta['description'] = 'Model for recognizing singer names in Hebrew song titles'
    nlp.meta['author'] = 'nhlocal'
    nlp.meta['email'] = 'nh.local11@gmail.com'
    nlp.meta['license'] = 'MIT'
    nlp.meta['tags'] = ['NER', 'Hebrew', 'Singer', 'Named Entity Recognition', 'Text Classification']

    model_name_path = os.path.join(os.getenv('GITHUB_WORKSPACE', '.'), "machine-learn", "model_name.txt")
    if os.path.exists(model_name_path):
        with open(model_name_path, 'r', encoding='utf-8') as f:
            model_name = f.read().strip()
            nlp.to_disk(model_name)
            print(f'# Model saved as {model_name}')
    else:
        print(f"Error: {model_name_path} not found.")
else:
    print("Error: Best model not found.")

# Save iteration data to a JSON file
iteration_data_path = os.path.join(os.getenv('GITHUB_WORKSPACE', '.'), 'machine-learn', 'iteration_data.json')
try:
    with open(iteration_data_path, 'w', encoding='utf-8') as f:
        json.dump(iteration_data, f, ensure_ascii=False, indent=2)
except Exception as e:
    print(f'Error saving iteration data to a JSON file: {e}')