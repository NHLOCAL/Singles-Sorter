import csv
import spacy
from spacy.training.example import Example
import random

# Load the pre-trained model
nlp = spacy.load('custom_ner_model')

# Access the NER component
ner = nlp.get_pipe("ner")

# Load training data from CSV file
def load_training_data(file_path):
    training_data = []
    with open(file_path, 'r', newline='', encoding='utf-8') as csvfile:
        reader = csv.DictReader(csvfile)
        for row in reader:
            training_data.append((row['prompt'], row['answer'], int(row['correct'])))
    return training_data

# Prepare training data with corrections
training_file = 'training_data2.csv'  # Replace with your updated CSV file
training_data = load_training_data(training_file)


random.shuffle(training_data)

# Initialize a dictionary to store the training data with feedback
training_data_dict = {}

for _ in range(1):  # You can adjust the number of iterations
    random.shuffle(training_data)
    losses = {}
    for prompt, expected_answer, correct in training_data:
        
        # Use the current model to predict the answer
        doc = nlp(prompt)
        
        # Extract the predicted entities from the Doc object
        predicted_entities = [(ent.start_char, ent.end_char, ent.label_) for ent in doc.ents]
        
        # Determine if the predicted answer is correct
        is_correct = int(predicted_entities == [(0, len(prompt), 'PER')]) if correct else 0

        # If prompt is already in the dictionary, append the feedback to the existing list
        if prompt in training_data_dict:
            training_data_dict[prompt].append({'entities': predicted_entities if is_correct else []})
        # If prompt is not in the dictionary, create a new list and add the feedback
        else:
            training_data_dict[prompt] = [{'entities': predicted_entities if is_correct else []}]

# Convert the dictionary to a list of tuples
training_data_with_feedback = [(prompt, feedback_list) for prompt, feedback_list in training_data_dict.items()]

print(training_data_with_feedback)

# Train the NER component with the training data
optimizer = nlp.begin_training()

for _ in range(1):
    losses = {}
    for text, annotations in training_data_with_feedback:
        example = Example.from_dict(nlp.make_doc(text), annotations)
        ner.update([example], drop=0.5, sgd=optimizer, losses=losses)

    print(losses)


# Save the corrected model to a new model file
nlp.to_disk("improved_model")
