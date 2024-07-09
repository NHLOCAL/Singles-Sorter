import csv
import random
from collections import Counter
import re

def load_singers(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        return [line.strip() for line in f]

def load_songs(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        return [line.strip() for line in f]

def find_singer(song, singers):
    for singer in singers:
        if singer in song:
            return singer
    return None

def count_singers(songs, singers):
    singer_counter = Counter()
    for song in songs:
        singer = find_singer(song, singers)
        if singer:
            singer_counter[singer] += 1
    return singer_counter

def replace_frequent_singers(songs, singer_counter, all_singers):
    new_songs = []
    for song in songs:
        current_singer = find_singer(song, all_singers)
        if current_singer and singer_counter[current_singer] > 10:
            new_singer = random.choice(all_singers)
            new_song = re.sub(r'\b' + re.escape(current_singer) + r'\b', new_singer, song)
            new_songs.append(new_song)
            print(f"החלפת '{current_singer}' ב-'{new_singer}' בשיר '{song}'")
        else:
            new_songs.append(song)
    return new_songs

def main():
    print("טוען את רשימת הזמרים...")
    all_singers = load_singers('singers_list.csv')
    print(f"נטענו {len(all_singers)} זמרים")

    print("טוען את רשימת השירים...")
    songs = load_songs('list_all_songs_clean.txt')
    print(f"נטענו {len(songs)} שירים")

    print("סופר את מספר ההופעות של כל זמר...")
    singer_counter = count_singers(songs, all_singers)

    print("מחליף זמרים שמופיעים יותר מ-10 פעמים...")
    new_songs = replace_frequent_singers(songs, singer_counter, all_singers)

    print("שומר את הרשימה המעודכנת...")
    with open('list_all_songs_random.txt', 'w', encoding='utf-8') as f:
        for song in new_songs:
            f.write(f"{song}\n")

    print("הפעולה הושלמה בהצלחה!")

if __name__ == "__main__":
    main()