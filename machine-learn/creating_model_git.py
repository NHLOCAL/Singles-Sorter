import spacy
from spacy.lang.he import Hebrew
from spacy.tokenizer import Tokenizer
from spacy.training.example import Example
import json
import random

# Function to check alignment and fix misaligned entities
def check_alignment(nlp, text, entities):
    doc = nlp.make_doc(text)
    tags = spacy.training.offsets_to_biluo_tags(doc, entities)
    return list(zip(doc, tags))

# Function to convert the data into spaCy's training format
def convert_to_spacy_format(data, ent):
    examples = []
    for example in data:
        full_string = example['Column A']
        entity_string = example['Column B']
        start_position = full_string.find(entity_string)
        end_position = start_position + len(entity_string)
        entities = [(start_position, end_position, ent)]

        # Check alignment and ignore misaligned entities during training
        aligned_entities = check_alignment(nlp, full_string, entities)

        doc = nlp.make_doc(full_string)
        example = Example.from_dict(doc, {'entities': entities})
        examples.append(example)

    return examples


def custom_tokenizer(nlp):
    # Load the default tokenizer
    default_tokenizer = Tokenizer(nlp.vocab)
    nlp2 = Hebrew()
    # Define the custom tokenization rule for "ו" at the beginning of a word using regex
    prefixes = nlp2.Defaults.prefixes + [r'^(?!וו)ו']
    prefix_regex = spacy.util.compile_prefix_regex(prefixes)
    nlp2.tokenizer.prefix_search = prefix_regex.search
    return nlp2.tokenizer


# Load a blank spaCy model
nlp = spacy.blank("he")
nlp.tokenizer = custom_tokenizer(nlp)

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

# Training loop
iteration_data = {}
for itn in range(55):
    losses = {}
    for example in training_data:
        nlp.update([example], drop=0.5, losses=losses)
    print(str(itn) + ": " + str(losses))
    iteration_data[itn] = losses.copy()  # Save the losses for this iteration
    if int(losses['ner']) <= 2900:
        break

# read name of model
with open("/home/runner/work/Singles-Sorter/Singles-Sorter/machine-learn/model_name.txt", 'r', encoding='utf-8') as f:
    model_name = f.read()
    print(f'# {model_name}')

# Save iteration data to a JSON file
try:
    with open(f'/home/runner/work/Singles-Sorter/Singles-Sorter/machine-learn/{model_name}/iteration_data.json', 'w', encoding='utf-8') as f:
        json.dump(iteration_data, f, ensure_ascii=False, indent=2)
except:
    print('was error in Save iteration data to a JSON file')    
    
# Save the trained model to disk
nlp.meta['name'] = 'find_singer_heb'
nlp.to_disk(model_name)

# Load the trained model later
# loaded_nlp = spacy.load("custom_ner_model")
