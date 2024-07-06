import random
import nltk
from nltk.corpus import wordnet
from googletrans import Translator

nltk.download('wordnet')
nltk.download('averaged_perceptron_tagger')

def get_synonyms(word):
    synonyms = []
    for syn in wordnet.synsets(word, lang='heb'):
        for lemma in syn.lemmas(lang='heb'):
            synonyms.append(lemma.name())
    return list(set(synonyms))

def replace_with_synonym(text):
    words = text.split()
    for i, word in enumerate(words):
        synonyms = get_synonyms(word)
        if synonyms:
            words[i] = random.choice(synonyms)
    return ' '.join(words)

def swap_words(text):
    words = text.split()
    if len(words) > 1:
        i, j = random.sample(range(len(words)), 2)
        words[i], words[j] = words[j], words[i]
    return ' '.join(words)

def back_translation(text, target_lang='en'):
    translator = Translator()
    translated = translator.translate(text, dest=target_lang)
    back_translated = translator.translate(translated.text, dest='iw')
    return back_translated.text

def add_connectors(text):
    connectors = ['ו', 'או', 'אבל', 'לכן', 'כי']
    words = text.split()
    if len(words) > 2:
        insert_pos = random.randint(1, len(words) - 1)
        words.insert(insert_pos, random.choice(connectors))
    return ' '.join(words)

def augment_data(text, num_augmentations=5):
    augmented_data = [text]
    
    for _ in range(num_augmentations):
        augmentation_type = random.choice(['synonym', 'swap', 'back_translation', 'add_connectors'])
        
        if augmentation_type == 'synonym':
            augmented_text = replace_with_synonym(text)
        elif augmentation_type == 'swap':
            augmented_text = swap_words(text)
        elif augmentation_type == 'back_translation':
            augmented_text = back_translation(text)
        else:
            augmented_text = add_connectors(text)
        
        augmented_data.append(augmented_text)
    
    return augmented_data

# דוגמה לשימוש
original_data = [
    "הרב שלמה הערש אקשטיין ניגון ה'אני מאמין'",
    "'שיר הלל' ו'פרחי הרא''ם' ב''אנא בכח'' של עובדיה חממה",
    "נמואל הרוש & יאיר שובל בדואט מרענן ''אבא אוהב''",
    "שמוליק ינקוביץ מגיש נְיֶעט נְיֶעט נִיקַאוָוא",
    "גלעד פוטולסקי מארח את מיכאל שטרייכר בסינגל חדש ''ויזכו לראות בנים''"
]

augmented_dataset = []
for text in original_data:
    augmented_dataset.extend(augment_data(text, num_augmentations=3))

# הדפסת התוצאות
for i, text in enumerate(augmented_dataset):
    print(f"{i+1}. {text}")
