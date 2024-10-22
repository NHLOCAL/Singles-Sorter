def generate_chunks(start, end, chunk_size):
    """
    מחלק את הטווח הכולל לקטעים בגודל מוגדר.

    Args:
        start (int): תחילת הטווח הכולל.
        end (int): סוף הטווח הכולל.
        chunk_size (int): גודל כל קטע.

    Returns:
        list of tuples: רשימה של טווחים (תחילת קטע, סוף קטע).
    """
    chunks = []
    current = start
    while current <= end:
        chunk_end = current + chunk_size - 1
        if chunk_end > end:
            chunk_end = end
        chunks.append((current, chunk_end))
        current = chunk_end + 1
    return chunks

def main():
    # הגדרת פרמטרים כקבועים
    START = 1          # תחילת הטווח הכולל
    END = 17600        # סוף הטווח הכולל
    CHUNK_SIZE = 500   # גודל כל קטע
    OUTPUT_FILE = 'steps.txt'  # שם קובץ הפלט

    # יצירת הקטעים
    chunks = generate_chunks(START, END, CHUNK_SIZE)

    # כתיבה לקובץ
    with open(OUTPUT_FILE, 'w', encoding='utf-8') as f:
        for chunk_start, chunk_end in chunks:
            f.write(f"      - name: Run Python Script for lines {chunk_start}-{chunk_end}\n")
            f.write("        env:\n")
            f.write("          GEMINI_API_KEY: ${{ secrets.GEMINI_API_KEY }}\n")
            f.write("        run: |\n")
            f.write(f"          python machine-learn/scrape_data/level0/Gemini-synthetic/gemini_api_creating.py {chunk_start} {chunk_end}\n\n")

    print(f"הוקמו {len(chunks)} קטעים ונשמרו בקובץ '{OUTPUT_FILE}'.")

if __name__ == "__main__":
    main()
