import spacy
from spacy.lang.he import Hebrew
from spacy.tokenizer import Tokenizer
from spacy.training.example import Example
from spacy.lang.char_classes import LIST_PUNCT, LIST_ELLIPSES, LIST_QUOTES, LIST_CURRENCY, LIST_ICONS
import json
import random

# Function to check alignment and fix misaligned entities
def check_alignment(nlp, text, entities):
    doc = nlp.make_doc(text)
    tags = spacy.training.offsets_to_biluo_tags(doc, entities)
    return list(zip(doc, tags))


def custom_tokenizer(nlp):
    # Load the default tokenizer
    default_tokenizer = Tokenizer(nlp.vocab)
    nlp2 = Hebrew()
    
    # Custom infix patterns
    LIST_BREAKING_WORDS = [r'—', r'--', r'-', r'\+']
    LIST_AMPERSAND = [r"[\x2D&]"]
    LIST_MORE = [r"״", "\."]
    

    custom_patterns = (
        LIST_QUOTES +
        LIST_ELLIPSES +
        LIST_BREAKING_WORDS +
        LIST_AMPERSAND +
        LIST_CURRENCY +
        LIST_PUNCT +
        LIST_ICONS +
        LIST_MORE
    )
    
    # Define custom prefix, infix, and suffix patterns to split '-'
    # Define the custom tokenization rule for "ו" at the beginning of a word using regex
    prefixes =  nlp2.Defaults.prefixes + custom_patterns + [r'^(?!וו)ו']
    infixes = nlp2.Defaults.infixes + custom_patterns
    suffixes = nlp2.Defaults.suffixes + custom_patterns

    prefix_regex = spacy.util.compile_prefix_regex(prefixes)
    infix_regex = spacy.util.compile_infix_regex(infixes)
    suffix_regex = spacy.util.compile_suffix_regex(suffixes)

    nlp2.tokenizer.prefix_search = prefix_regex.search
    nlp2.tokenizer.infix_finditer = infix_regex.finditer
    nlp2.tokenizer.suffix_search = suffix_regex.search
    
    return nlp2.tokenizer
    

# Load a blank spaCy model
nlp = spacy.blank("he")
nlp.tokenizer = custom_tokenizer(nlp)

# Test the tokenizer
test_text = "תומר כהן- הישראלי הבכיר בלינקדין"
doc = nlp(test_text)
print([token.text for token in doc])


# Add the entity recognizer to the pipeline using its string name
ner = nlp.add_pipe("ner")
ner.add_label("SINGER")

# Load data from both JSON files
json_files = [
    '/home/runner/work/Singles-Sorter/Singles-Sorter/machine-learn/scrape_data/cleaned_new-data.json'
    # '/home/runner/work/Singles-Sorter/Singles-Sorter/machine-learn/scrape_data/cleaned_data.json',
]

training_data = []
for json_file in json_files:
    with open(json_file, 'r', encoding='utf-8') as f:
        data = json.load(f)
        for example_text, example_entities in data:
            entities = example_entities.get('entities', [])
            example = Example.from_dict(nlp.make_doc(example_text), {'entities': entities})
            training_data.append(example)

# Load the data from the JSON file
with open(json_file, 'r', encoding='utf-8') as f:
    data = json.load(f)

# Convert the data to spaCy format
training_data = []
for example_text, example_entities in data:
    entities = example_entities.get('entities', [])
    example = Example.from_dict(nlp.make_doc(example_text), {'entities': entities})
    training_data.append(example)

# Shuffle the training data
random.shuffle(training_data)

# Start training the spaCy model
nlp.begin_training()

# Early Stopping Parameters
patience = 5  # מספר האיטרציות לחכות לפני עצירת האימון אם אין שיפור
min_delta = 0.001  # השיפור המינימלי הנדרש כדי להיחשב שיפור
best_loss = float('inf')
patience_counter = 0

# Training loop
iteration_data = {}
batch_size = 32
for itn in range(55):
    losses = {}
    for i in range(0, len(training_data), batch_size):
        batch = training_data[i:i + batch_size]
        nlp.update(batch, drop=0.3, losses=losses)
    print(f"Iteration {itn}: {losses}")
    iteration_data[itn] = losses.copy()
    
    # Early Stopping Check
    current_loss = losses.get('ner', float('inf'))
    if current_loss < best_loss - min_delta:
        best_loss = current_loss
        patience_counter = 0
    else:
        patience_counter += 1
    
    if patience_counter >= patience:
        print(f"Early stopping at iteration {itn}")
        break

# read name of model
with open("/home/runner/work/Singles-Sorter/Singles-Sorter/machine-learn/model_name.txt", 'r', encoding='utf-8') as f:
    model_name = f.read()
    print(f'# {model_name}')

# Save iteration data to a JSON file
try:
    with open(f'/home/runner/work/Singles-Sorter/Singles-Sorter/machine-learn/iteration_data.json', 'w', encoding='utf-8') as f:
        json.dump(iteration_data, f, ensure_ascii=False, indent=2)
except Exception as e:
    print(f'was error in Save iteration data to a JSON file: {e}')    
    
# Save the trained model to disk
nlp.meta['name'] = 'find_singer_heb'
nlp.to_disk(model_name)