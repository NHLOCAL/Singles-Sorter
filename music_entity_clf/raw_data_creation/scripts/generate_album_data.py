import os
import music_tag
from jibrish_to_hebrew import fix_jibrish

def get_album_names(root_dir):
    album_names = set()
    audio_extensions = {'.mp3', '.flac', '.m4a', '.wav', '.ogg', '.wma', '.aac'}

    for subdir, _, files in os.walk(root_dir):
        for file in files:
            file_path = os.path.join(subdir, file)
            file_ext = os.path.splitext(file)[1].lower()

            if file_ext in audio_extensions:
                try:
                    f = music_tag.load_file(file_path)
                    album_name = str(f['album']).strip()
                    album_name = fix_jibrish(album_name, "heb")
                    if album_name.lower() not in ["single", "סינגל"] and album_name:
                        album_names.add(album_name)
                except Exception as e:
                    print(f"Error processing file {file_path}: {e}")

    return list(album_names)

def save_album_names_to_file(album_names, output_file):
    with open(output_file, 'a', encoding='utf-8') as f:
        for name in sorted(album_names):
            f.write(name + '\n')

if __name__ == "__main__":
    root_dir = r"D:\שמע"
    output_file = "album_names.txt"
    
    album_names = get_album_names(root_dir)
    save_album_names_to_file(album_names, output_file)
    print(f"Album names saved to {output_file}")
