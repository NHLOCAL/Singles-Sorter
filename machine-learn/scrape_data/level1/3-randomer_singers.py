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

def balance_singers(songs, all_singers, max_appearances=1, num_rounds=5):
    for round in range(num_rounds):
        print(f"סבב איזון {round + 1}/{num_rounds}")
        singer_counter = count_singers(songs, all_singers)
        unused_singers = set(all_singers) - set(singer_counter.keys())
        
        new_songs = []
        for song in songs:
            current_singer = find_singer(song, all_singers)
            if current_singer and singer_counter[current_singer] > max_appearances:
                if unused_singers:
                    new_singer = random.choice(list(unused_singers))
                    unused_singers.remove(new_singer)
                else:
                    new_singer = min(singer_counter, key=singer_counter.get)
                
                new_song = re.sub(r'\b' + re.escape(current_singer) + r'\b', new_singer, song)
                new_songs.append(new_song)
                singer_counter[current_singer] -= 1
                singer_counter[new_singer] += 1
                print(f"החלפת '{current_singer}' ב-'{new_singer}' בשיר '{song}'")
            else:
                new_songs.append(song)
        
        songs = new_songs
    
    return songs

def main():
    print("טוען את רשימת הזמרים...")
    all_singers = load_singers('singers_list.csv')
    print(f"נטענו {len(all_singers)} זמרים")

    print("טוען את רשימת השירים...")
    songs = load_songs('list_all_songs_clean.txt')
    print(f"נטענו {len(songs)} שירים")

    print("מאזן את הופעות הזמרים...")
    balanced_songs = balance_singers(songs, all_singers)

    print("שומר את הרשימה המאוזנת...")
    with open('list_all_songs_balanced.txt', 'w', encoding='utf-8') as f:
        for song in balanced_songs:
            f.write(f"{song}\n")

    print("הפעולה הושלמה בהצלחה!")

    # הצגת סטטיסטיקות
    final_counter = count_singers(balanced_songs, all_singers)
    print("\nסטטיסטיקות סופיות:")
    print(f"מספר זמרים ייחודיים: {len(final_counter)}")
    print(f"מספר הופעות מינימלי: {min(final_counter.values())}")
    print(f"מספר הופעות מקסימלי: {max(final_counter.values())}")
    print(f"ממוצע הופעות: {sum(final_counter.values()) / len(final_counter):.2f}")

if __name__ == "__main__":
    main()