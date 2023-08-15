import pandas as pd

def calculate_positions(full_string, person_string):
    start_idx = full_string.find(person_string)
    end_idx = start_idx + len(person_string)
    return start_idx, end_idx

# Replace 'your_data.csv' with the actual path to your CSV file
input_csv = 'find_singers_model.csv'
output_csv = 'output_data.csv'

# Read the CSV file into a Pandas DataFrame
df = pd.read_csv(input_csv)

# Create empty lists to store start and end positions
start_positions = []
end_positions = []

# Iterate through each row in the DataFrame
for index, row in df.iterrows():
    full_string = row['Column A']  # Replace 'Column A' with the name of your text column
    person_string = row['Column B']  # Replace 'Column B' with the name of your PERSON column

    # Calculate the start and end positions of the PERSON string in the full string
    start_idx, end_idx = calculate_positions(full_string, person_string)
    
    start_positions.append(start_idx)
    end_positions.append(end_idx)

# Add start and end positions to the DataFrame
df['Start Position'] = start_positions
df['End Position'] = end_positions

# Save the updated DataFrame to a new CSV file
df.to_csv(output_csv, index=False)
