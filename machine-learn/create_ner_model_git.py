import spacy
from spacy.lang.he import Hebrew
from spacy.tokenizer import Tokenizer
from spacy.training.example import Example
import pandas as pd
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

# Define your custom label scheme (e.g., PERSON, ORG, LOC, etc.)
ner.add_label("SINGER")
ner.add_label("OTHER")

# Replace 'training_data.csv' with the actual path to your positive training CSV file
positive_csv = '/home/runner/work/Singles-Sorter/Singles-Sorter/machine-learn/training_data.csv'

# Replace 'negative_data.csv' with the actual path to your negative training CSV file
negative_csv = '/home/runner/work/Singles-Sorter/Singles-Sorter/machine-learn/negative_data.csv'

# Read the positive and negative CSV files into Pandas DataFrames
df_positive = pd.read_csv(positive_csv)
df_negative = pd.read_csv(negative_csv)

# Convert the data to spaCy format for both positive and negative samples
training_data_positive = convert_to_spacy_format(df_positive.to_dict('records'), 'SINGER')
training_data_negative = convert_to_spacy_format(df_negative.to_dict('records'), 'OTHER')

# Combine positive and negative samples and shuffle
training_data_combined = training_data_positive + training_data_negative
random.shuffle(training_data_combined)

# Start training the spaCy model
nlp.begin_training()

# Training loop
for itn in range(25):
    losses = {}
    for example in training_data_combined:
        nlp.update([example], drop=0.3, losses=losses)
    if int(losses['ner']) <= 3500:
        break
    print(str(itn) + ": " + str(losses))

# Save the trained model to disk
nlp.meta['name'] = 'find_singer_heb'
nlp.to_disk("custom_ner_model")