import csv
import random

# Load singers from the CSV file
def load_singers(csv_file):
    with open(csv_file, 'r', encoding='utf-8') as file:
        reader = csv.reader(file)
        singers = [row[0] for row in reader]
    return singers

# Replace SINGER tags in text file
def replace_singer_tags(text_file, singers):
    with open(text_file, 'r', encoding='utf-8') as file:
        lines = file.readlines()

    replaced_lines = []
    for line in lines:
        while 'SINGER' in line:
            random_singer = random.choice(singers)
            line = line.replace('SINGER', random_singer, 1)
        replaced_lines.append(line)

    return replaced_lines

# Write the updated lines back to a new file
def write_updated_file(output_file, replaced_lines):
    with open(output_file, 'w', encoding='utf-8') as file:
        file.writelines(replaced_lines)

# Main function
def main():
    singers_csv = 'singers_list.csv'
    input_text_file = 'all_songs_synthetic.txt'
    output_text_file = 'all_songs2.txt'

    singers = load_singers(singers_csv)
    replaced_lines = replace_singer_tags(input_text_file, singers)
    write_updated_file(output_text_file, replaced_lines)
    print(f"Updated file saved as {output_text_file}")

if __name__ == "__main__":
    main()
