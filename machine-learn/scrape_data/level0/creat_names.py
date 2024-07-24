import hebrew_names as hn
from collections import Counter

def generate_unique_names(count, max_repeats=10, max_attempts=10000):
    names = []
    first_names = Counter()
    last_names = Counter()
    skipped_names = 0
    total_attempts = 0

    while len(names) < count and total_attempts < max_attempts:
        total_attempts += 1
        try:
            full_name = hn.get_full_name(ethnicity='jew', gender='male')
            name_parts = full_name.split()
            
            if len(name_parts) != 2:
                skipped_names += 1
                continue
            
            first_name, last_name = name_parts

            if (first_names[first_name] < max_repeats and 
                last_names[last_name] < max_repeats):
                names.append(full_name)
                first_names[first_name] += 1
                last_names[last_name] += 1
                print(f"{len(names)}. {full_name}")
        except Exception as e:
            print(f"Error occurred: {e}. Skipping this name.")
            skipped_names += 1

    print(f"\nTotal names skipped: {skipped_names}")
    print(f"Total attempts: {total_attempts}")
    if total_attempts >= max_attempts:
        print(f"Warning: Reached maximum number of attempts ({max_attempts}) before generating {count} names.")
    return names

names = generate_unique_names(5000, max_repeats=10, max_attempts=25000)

output_file = "random_names.txt"

with open(output_file, 'w', encoding='utf-8') as f:
    for i, name in enumerate(names, 1):
        f.write(f"{name}\n")

print(f"\nTotal unique names generated: {len(names)}")
print(f"Unique first names: {len(set(name.split()[0] for name in names))}")
print(f"Unique last names: {len(set(name.split()[1] for name in names))}")