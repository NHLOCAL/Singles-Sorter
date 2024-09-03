import re
import chardet
import random

def detect_encoding(file_path):
    """
    מזהה את קידוד הקובץ.
    """
    with open(file_path, 'rb') as f:
        rawdata = f.read()
    return chardet.detect(rawdata)['encoding']

def extract_and_chunk_text(file_list, output_file):
    """
    מחלץ טקסט מקבצי טקסט, חותך אותו לקטעי משפטים באורך 2-4 מילים, 
    מסיר מרכאות כפולות, ומערבב את הרשימה בסדר אקראי.
    """

    all_chunks = []

    for file_name in file_list:
        encoding = detect_encoding(file_name)
        with open(file_name, 'r', encoding=encoding) as f:
            text = f.read()
            
            # מחלק את הטקסט למשפטים
            sentences = re.split(r'(?<=[.!?])\s+', text)
            
            for sentence in sentences:
                # מחלק כל משפט למילים
                words = sentence.split()
                
                # יוצר קטעים באורך 2-4 מילים
                i = 0
                while i < len(words) - 1:
                    chunk_length = random.randint(2, min(4, len(words) - i))
                    chunk = " ".join(words[i:i + chunk_length])
                    
                    # מסיר מרכאות כפולות
                    chunk = chunk.replace('"', '')
                    
                    all_chunks.append(chunk)
                    i += chunk_length

    # מערבב את הרשימה בסדר אקראי
    random.shuffle(all_chunks)

    # שומר את קטעי הטקסט לקובץ חדש
    with open(output_file, 'w', encoding='utf-8') as f:
        for chunk in all_chunks:
            f.write(chunk + '\n')

# דוגמא לשימוש:
file_list = [
    "D:\מסמכים וספרים\לנגן\חזל - ברכות.txt",
    "D:\מסמכים וספרים\לנגן\מילים נרדפות.txt",
    "D:\מסמכים וספרים\לנגן\מספר הטלפון שלך.txt",
    "D:\מסמכים וספרים\לנגן\מעשה אברהם אבינו.txt",
    "D:\מסמכים וספרים\לנגן\על האש.txt",
    "D:\מסמכים וספרים\לנגן\פלאפון.txt",
    "D:\מסמכים וספרים\לנגן\ציפורים שלא עפות.txt",
    "D:\מסמכים וספרים\לנגן\פקק.txt",
    "D:\מסמכים וספרים\לנגן\חול (2)\העיתונאי 1.txt",
    "D:\מסמכים וספרים\לנגן\חול (2)\השנורר המיליונר.txt",
    "D:\מסמכים וספרים\לנגן\חול (2)\מעניין\הכובע של מרים.txt"



] 
output_file = 'chunked_text.txt'
extract_and_chunk_text(file_list, output_file)