# -*- coding: utf-8 -*-

import csv
import os
from find_singer_names import files_list


input_array = files_list(r'C:\Users\משתמש\Documents\song_list')


# Filter out non-empty lists from the input array
non_empty_lists = [lst for lst in input_array if lst is not None]


singer_names_csv = 'singers_list.csv'

with open(singer_names_csv, mode='r', newline='', encoding='utf-8') as file:
    reader = csv.reader(file)
    singer_names = [row[0] for row in reader]

non_empty_lists = [obj for obj in non_empty_lists if obj[0] not in singer_names]

# Create a numbered list of non-empty lists
numbered_list = [(idx + 1, lst) for idx, lst in enumerate(non_empty_lists)]

# Display the numbered list
for idx, lst in numbered_list:
    print(f"{idx}. {lst}")

# Select objects by number (e.g., 5 and 7)
selected_numbers = input()
selected_numbers = selected_numbers.split()
selected_numbers = [int(n) for n in selected_numbers]


print(selected_numbers)

# ############################


# Create a new list with selected objects
selected_objects = [numbered_list[idx - 1][1] for idx in selected_numbers]


# Create a list to hold the rows for the CSV
csv_rows = []

# Iterate through the selected objects and extract information
for obj in selected_objects:
    singer_name = obj[0]  # Assuming the singer's name is the first element
    full_string = obj[1]  # Assuming the full string is the second element
    start_position = full_string.find(singer_name)
    end_position = start_position + len(singer_name)
    
    # Append the extracted information as a row to the CSV list
    csv_rows.append([full_string, singer_name, start_position, end_position])

# Define the CSV file name
output_csv = 'negative_data.csv'

# Write the CSV file with the specified structure
with open(output_csv, mode='a', newline='', encoding='utf-8') as file:
    writer = csv.writer(file)
    # writer.writerow(['Column A', 'Column B', 'Start Position', 'End Position'])  # Write header
    writer.writerows(csv_rows)  # Write data rows

print(f"CSV dataset saved to {output_csv}")
