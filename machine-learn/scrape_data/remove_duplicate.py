import csv
from collections import defaultdict

# Read the CSV file and store data in a dictionary
data = defaultdict(list)
with open('training_data.csv', newline='', encoding='utf-8') as csvfile:
    reader = csv.reader(csvfile)
    next(reader)  # Skip header row
    for row in reader:
        name = row[0]
        entity = row[1]
        data[entity].append(row)

# Filter data to keep up to 3 instances of each entity
filtered_data = []
for entity, rows in data.items():
    filtered_rows = rows[:3]  # Keep the first 3 instances
    filtered_data.extend(filtered_rows)

# Write the filtered data back to a new CSV file
with open('filtered_file.csv', mode='w', newline='', encoding='utf-8') as csvfile:
    writer = csv.writer(csvfile)
    writer.writerow(['Name', 'Entity', 'Column C', 'Column D'])  # Write header
    writer.writerows(filtered_data)
