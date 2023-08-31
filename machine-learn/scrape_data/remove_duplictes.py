def remove_duplicates_from_text_file(input_file, output_file):
    unique_items = set()
    with open(input_file, mode='r', encoding='utf-8') as infile:
        lines = infile.readlines()
        for line in lines:
            item = line.strip()
            if item not in unique_items:
                unique_items.add(item)

    with open(output_file, mode='w', encoding='utf-8') as outfile:
        for item in unique_items:
            outfile.write(item + '\n')

input_text_file = 'songs_list.txt'
output_text_file = 'output_list.txt'

remove_duplicates_from_text_file(input_text_file, output_text_file)
print(f"Duplicate-free items saved to {output_text_file}")
