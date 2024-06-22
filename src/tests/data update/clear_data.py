import csv

def remove_duplicates(names_file, singer_file, output_file, removed_file):
    # Read the names from the names.txt file
    with open(names_file, 'r', encoding='utf-8') as file:
        names = file.read().splitlines()

    # Read the singer names from the singer-list.csv file
    with open(singer_file, 'r') as file:
        csv_reader = csv.reader(file)
        singers = set(row[0] for row in csv_reader)

    # Remove duplicate names and collect removed names
    filtered_names = []
    removed_names = []
    for name in names:
        if name in singers:
            removed_names.append(name)
        else:
            filtered_names.append(name)

    # Write the remaining names to the output file
    with open(output_file, 'w') as file:
        file.write('\n'.join(filtered_names))

    # Write the removed names to the removed file
    with open(removed_file, 'w') as file:
        file.write('\n'.join(removed_names))

# Provide the file paths
names_file = 'names.txt'
singer_file = 'singer-list.csv'
output_file = 'filtered_names.txt'
removed_file = 'removed_names.txt'

# Call the function
remove_duplicates(names_file, singer_file, output_file, removed_file)
