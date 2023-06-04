def sort_names(input_file, sorted_file, one_word_file, three_or_more_words_file):
    with open(input_file, 'r') as file:
        names = file.read().splitlines()

    sorted_names = []
    one_word_names = []
    three_or_more_words_names = []

    for name in names:
        if len(name.split()) != 2:
            if len(name.split()) == 1:
                one_word_names.append(name)
            else:
                three_or_more_words_names.append(name)
        else:
            sorted_names.append(name)

    with open(sorted_file, 'w') as file:
        file.write('\n'.join(sorted_names))

    with open(one_word_file, 'w') as file:
        file.write('\n'.join(one_word_names))

    with open(three_or_more_words_file, 'w') as file:
        file.write('\n'.join(three_or_more_words_names))

    print(f"Names sorted. Sorted names are saved in '{sorted_file}'.")
    print(f"Names with only one word are saved in '{one_word_file}'.")
    print(f"Names with three or more words are saved in '{three_or_more_words_file}'.")

    return sorted_names, one_word_names, three_or_more_words_names


# Usage example
input_file = 'names.txt'
sorted_file = 'sorted_names.txt'
one_word_file = 'one_word_names.txt'
three_or_more_words_file = 'three_or_more_words_names.txt'

sorted_names, one_word_names, three_or_more_words_names = sort_names(
    input_file, sorted_file, one_word_file, three_or_more_words_file
)

print("Sorted names:", sorted_names)
print("Names with only one word:", one_word_names)
print("Names with three or more words:", three_or_more_words_names)
