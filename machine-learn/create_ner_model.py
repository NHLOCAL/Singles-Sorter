import spacy
from spacy.training.example import Example
import pandas as pd
import random


# Function to check alignment and fix misaligned entities
def check_alignment(nlp, text, entities):
    doc = nlp.make_doc(text)
    tags = spacy.training.offsets_to_biluo_tags(doc, entities)
    return list(zip(doc, tags))

# Function to convert the data into spaCy's training format
def convert_to_spacy_format(data):
    examples = []
    for example in data:
        full_string = example['Column A']  # Replace 'Column_A' with the correct column name for the text
        entity_string = example['Column B']    # Replace 'Column_B' with the correct column name for the entities
        start_position = full_string.find(entity_string)
        end_position = start_position + len(entity_string)
        entities = [(start_position, end_position, 'SINGER')]  # Adjust the label ('PER') as needed

        # Check alignment and ignore misaligned entities during training
        aligned_entities = check_alignment(nlp, full_string, entities)
        if '-' in [tag for _, tag in aligned_entities]:
            continue

        doc = nlp.make_doc(full_string)
        example = Example.from_dict(doc, {'entities': entities})
        examples.append(example)

    return examples


# Load a blank spaCy model
nlp = spacy.blank("he")

# Add the entity recognizer to the pipeline using its string name
ner = nlp.add_pipe("ner")

# Define your custom label scheme (e.g., PERSON, ORG, LOC, etc.)
ner.add_label("SINGER")

# Replace 'output_data.csv' with the actual path to your output CSV file
output_csv = '/home/runner/work/Singles-Sorter/Singles-Sorter/machine-learn/training_data.csv'

# Read the output CSV file into a Pandas DataFrame
df = pd.read_csv(output_csv)

# Convert the data to spaCy format
training_data = convert_to_spacy_format(df.to_dict('records'))  # Convert DataFrame to a list of dictionaries
random.shuffle(training_data)


# Start training the spaCy model
nlp.begin_training()

# Training loop
for itn in range(400):
    losses = {}
    for example in training_data:
        nlp.update([example], drop=0.35, losses=losses)
    if int(losses['ner']) <= 150: break
    print(str(itn) + ": " + str(losses))



# Save the trained model to disk
nlp.meta['name'] = 'find_singer_heb'
nlp.to_disk("custom_ner_model")

# Load the trained model later
# loaded_nlp = spacy.load("custom_ner_model")
