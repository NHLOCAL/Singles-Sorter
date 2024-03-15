import json

# Load the JSON file
input_file = 'data.json'
output_file = 'cleaned_data.json'

with open(input_file, 'r', encoding='utf-8') as f:
    data = json.load(f)

# Function to check for conflicting values in a list of entities
def has_conflicts(entities):
    positions = {}
    for entity in entities:
        start, end, _ = entity
        for i in range(start, end):
            if i in positions:
                return True
            positions[i] = True
    return False

# Function to filter out conflicting sections from entities
def filter_conflicts(entities):
    filtered_entities = []
    positions = {}
    for entity in entities:
        start, end, _ = entity
        conflicting = False
        for i in range(start, end):
            if i in positions:
                conflicting = True
                break
            positions[i] = True
        if not conflicting:
            filtered_entities.append(entity)
    return filtered_entities

# Iterate through each example
cleaned_data = []
for song_name, example in data:
    entities = example['entities']
    if has_conflicts(entities):
        # Remove conflicting sections
        filtered_entities = filter_conflicts(entities)
        if filtered_entities:
            cleaned_data.append((song_name, {'entities': filtered_entities}))
    else:
        cleaned_data.append((song_name, example))

# Write the cleaned dataset to the JSON file
with open(output_file, 'w', encoding='utf-8') as f:
    json.dump(cleaned_data, f, ensure_ascii=False, indent=2)

print(f'Cleaned data saved to {output_file}')
