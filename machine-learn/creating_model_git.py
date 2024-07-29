import spacy
from spacy.lang.he import Hebrew
from spacy.tokenizer import Tokenizer
from spacy.training.example import Example
from spacy.lang.char_classes import LIST_PUNCT, LIST_ELLIPSES, LIST_QUOTES, LIST_CURRENCY, LIST_ICONS
from spacy.util import minibatch, compounding
import json
import random

def custom_tokenizer(nlp):
    default_tokenizer = Tokenizer(nlp.vocab)
    nlp2 = Hebrew()
    
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

test_text = "תומר כהן- הישראלי הבכיר בלינקדין"
doc = nlp(test_text)
print([token.text for token in doc])

ner = nlp.add_pipe("ner")
ner.add_label("SINGER")

json_files = [
    '/home/runner/work/Singles-Sorter/Singles-Sorter/machine-learn/scrape_data/cleaned_new-data.json'
]

training_data = []
for json_file in json_files:
    with open(json_file, 'r', encoding='utf-8') as f:
        data = json.load(f)
        for example_text, example_entities in data:
            entities = example_entities.get('entities', [])
            example = Example.from_dict(nlp.make_doc(example_text), {'entities': entities})
            training_data.append(example)

random.shuffle(training_data)

nlp.begin_training()

patience = 5
min_delta = 0.001
best_loss = float('inf')
patience_counter = 0
best_model_path = "/home/runner/work/Singles-Sorter/Singles-Sorter/machine-learn/best_model"

n_iter = 100
batch_sizes = compounding(16.0, 64.0, 1.001)
iteration_data = {}
#initial_lr = 0.001  # שיעור למידה התחלתי
#lr_decay = 0.95  # קצב דעיכת שיעור הלמידה
# optimizer.learn_rate = initial_lr

for itn in range(n_iter):
    random.shuffle(training_data)
    batches = minibatch(training_data, size=batch_sizes)
    losses = {}
    for batch in batches:
        nlp.update(batch, drop=0.4, losses=losses)
    print(f"Iteration {itn}, Losses: {losses}")
    iteration_data[itn] = losses.copy()
    
    current_loss = losses.get('ner', float('inf'))
    if current_loss < best_loss - min_delta:
        best_loss = current_loss
        patience_counter = 0
        # Save the best model
        nlp.to_disk(best_model_path)
    else:
        patience_counter += 1
    
    if patience_counter >= patience:
        print(f"Early stopping at iteration {itn}")
        break

    # עדכון שיעור הלמידה
    #optimizer.learn_rate *= lr_decay

with open("/home/runner/work/Singles-Sorter/Singles-Sorter/machine-learn/model_name.txt", 'r', encoding='utf-8') as f:
    model_name = f.read()
    print(f'# {model_name}')

try:
    with open(f'/home/runner/work/Singles-Sorter/Singles-Sorter/machine-learn/iteration_data.json', 'w', encoding='utf-8') as f:
        json.dump(iteration_data, f, ensure_ascii=False, indent=2)
except Exception as e:
    print(f'was error in Save iteration data to a JSON file: {e}')

# Load the best model before saving with the final name
nlp = spacy.load(best_model_path)
nlp.meta['name'] = 'find_singer_heb'
nlp.to_disk(model_name)
