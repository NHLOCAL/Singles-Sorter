import csv
import random
from collections import Counter
import re

class SingerSelector:
    def __init__(self, singers):
        self.all_singers = singers
        self.reset()

    def reset(self):
        self.available_singers = self.all_singers.copy()
        random.shuffle(self.available_singers)

    def get_singer(self):
        if not self.available_singers:
            self.reset()
        new_singer = self.available_singers.pop()
        # Ensure the singer is not 'SINGER'
        while new_singer == "SINGER":
            if not self.available_singers:  # Reset again if needed
                self.reset()
            new_singer = self.available_singers.pop()
        return new_singer

def load_singers(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        return [line.strip() for line in f]

def load_songs(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        return [line.strip() for line in f]

def find_singers(song, singers):
    return [singer for singer in singers if singer in song]

def count_singers(songs, singers):
    singer_counter = Counter()
    for song in songs:
        for singer in find_singers(song, singers):
            singer_counter[singer] += 1
    return singer_counter

def replace_singer_and_balance(songs, all_singers, max_appearances=15, num_rounds=3):
    singer_selector = SingerSelector(all_singers)
    
    for round in range(num_rounds):
        print(f"סבב איזון והחלפה {round + 1}/{num_rounds}")
        singer_counter = count_singers(songs, all_singers)
        
        new_songs = []
        for song in songs:
            new_song = song
            
            # החלפת SINGER בשם זמר רנדומלי
            while 'SINGER' in new_song:
                new_singer = singer_selector.get_singer()
                new_song = new_song.replace('SINGER', new_singer, 1)
                singer_counter[new_singer] += 1
            
            # איזון הופעות הזמרים
            current_singers = find_singers(new_song, all_singers)
            for current_singer in current_singers:
                if singer_counter[current_singer] > max_appearances:
                    new_singer = singer_selector.get_singer()
                    while new_singer in current_singers:
                        new_singer = singer_selector.get_singer()
                    
                    new_song = re.sub(r'\b' + re.escape(current_singer) + r'\b', new_singer, new_song)
                    singer_counter[current_singer] -= 1
                    singer_counter[new_singer] += 1
            
            new_songs.append(new_song)
        
        songs = new_songs
        singer_selector.reset()
    
    return songs

def print_top_singers(singer_counter, n=10):
    print(f"\n{n} הזמרים עם מספר ההופעות הגבוה ביותר:")
    for singer, count in singer_counter.most_common(n):
        print(f"{singer}: {count} הופעות")

def main():
    print("טוען את רשימת הזמרים...")
    all_singers = load_singers('singers_list.csv')
    print(f"נטענו {len(all_singers)} זמרים")

    print("טוען את רשימת השירים...")
    songs = load_songs('list_all_songs_with_singer.txt')  # שימו לב לשינוי בשם הקובץ
    print(f"נטענו {len(songs)} שירים")

    print("מחליף SINGER בשמות זמרים ומאזן את ההופעות...")
    balanced_songs = replace_singer_and_balance(songs, all_singers)

    print("שומר את הרשימה המאוזנת...")
    with open('list_all_songs_random.txt', 'w', encoding='utf-8') as f:
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

    # הדפסת 10 הזמרים המובילים
    print_top_singers(final_counter)

if __name__ == "__main__":
    main()
