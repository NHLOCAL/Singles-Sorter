from spacy import load

# read name of model
with open("model_name.txt", 'r', encoding='utf-8') as f:
    model_name = f.read().strip()  # הסר רווחים קודמים אחרי קריאה
    print(f'# {model_name}')

# Load your trained model
nlp = load(f'models/{model_name}')

def machine_learn(text):
    # Process the text with the loaded model
    doc = nlp(text)

    # Access the entities recognized by the model
    entities = []
    for entity in doc.ents:
        if entity.label_ == 'SINGER':
            entities.append(entity.text)  # הוסף רק את שם הזמר

    return entities

def value_generator(file_path):
    with open(file_path, 'r', encoding='utf-8') as file:
        for line in file:
            yield line.strip()

def remove_duplicates(file_path):
    # קרא את כל השמות מהקובץ
    with open(file_path, 'r', encoding='utf-8') as file:
        lines = file.readlines()

    # הסר כפילויות ושמור על סדר השמות
    unique_lines = sorted(set(line.strip() for line in lines if line.strip()))

    # כתוב את השמות הייחודיים חזרה לקובץ
    with open(file_path, 'w', encoding='utf-8') as file:
        for line in unique_lines:
            file.write(f"{line}\n")

if __name__ == "__main__":

    # Read and process texts from the file
    file_path = r"scrape_data\level1\list_unknown_songs.txt"
    output_file_path = 'singer_names.txt'

    with open(output_file_path, 'w', encoding='utf-8') as output_file:
        for value in value_generator(file_path):
            entities = machine_learn(value)
            if entities:
                for entity in entities:
                    output_file.write(f"{entity}\n")

    # הסר כפילויות מהקובץ
    remove_duplicates(output_file_path)

    print(f"Unique SINGER names have been written to {output_file_path}")
