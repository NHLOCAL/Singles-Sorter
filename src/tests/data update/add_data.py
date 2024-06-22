import csv

def create_csv_from_names(names_file, output_file):
    # Read the names from the sorted_names.txt file
    with open(names_file, 'r') as file:
        names = file.read().splitlines()

    # Prepare data for CSV writing
    data = [[name] for name in names]

    # Write the data to the output CSV file
    with open(output_file, 'w', newline='') as file:
        csv_writer = csv.writer(file)
        csv_writer.writerows(data)

# Provide the file paths
names_file = 'sorted_names.txt'
output_file = 'names.csv'

# Call the function
create_csv_from_names(names_file, output_file)
