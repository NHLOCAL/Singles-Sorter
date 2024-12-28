import os
import json
from pathlib import Path
from music_tag import load_file
from jibrish_to_hebrew import fix_jibrish, check_jibrish

SUPPORTED_EXTENSIONS = {'.mp3', '.flac', '.wav', '.aac', '.m4a', '.ogg', '.alac'}

def get_metadata(file_path):
    """Extract metadata from an audio file and convert to proper Hebrew."""
    try:
        metadata = load_file(file_path)
        return {
            'title': fix_jibrish(metadata['title'].value, "heb") if check_jibrish(metadata['title'].value) else metadata['title'].value,
            'artist': fix_jibrish(metadata['artist'].value, "heb") if check_jibrish(metadata['artist'].value) else metadata['artist'].value,
            'album': fix_jibrish(metadata['album'].value, "heb") if check_jibrish(metadata['album'].value) else metadata['album'].value,
            'genre': fix_jibrish(metadata['genre'].value, "heb") if check_jibrish(metadata['genre'].value) else metadata['genre'].value,
            'year': metadata['year'].value
        }
    except Exception as e:
        print(f"Error extracting metadata from {file_path}: {e}")
        return {}

def scan_directory(directory):
    """Scan a directory and generate JSON files with file details and metadata."""
    directory = Path(directory)

    if not directory.is_dir():
        print(f"Invalid directory: {directory}")
        return

    result = {}

    for root, _, files in os.walk(directory):
        relative_path = os.path.relpath(root, directory)
        result[relative_path] = []

        for file in files:
            file_path = Path(root) / file
            file_info = {
                'name': file,
                'path': str(file_path),
                'is_audio': file_path.suffix.lower() in SUPPORTED_EXTENSIONS
            }

            if file_info['is_audio']:
                file_info['metadata'] = get_metadata(file_path)

            result[relative_path].append(file_info)

    # Save the result as a JSON file
    output_file = directory / "directory_structure.json"
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(result, f, indent=4, ensure_ascii=False)

    print(f"JSON file created: {output_file}")

if __name__ == "__main__":
    directory_to_scan = input("Enter the directory path to scan: ").strip()
    scan_directory(directory_to_scan)
