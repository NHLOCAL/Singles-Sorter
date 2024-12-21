import spacy
from spacy.lang.he import Hebrew
from spacy.tokenizer import Tokenizer
from spacy.training.example import Example
from spacy.lang.char_classes import LIST_PUNCT, LIST_ELLIPSES, LIST_QUOTES, LIST_CURRENCY, LIST_ICONS
from spacy.util import minibatch, compounding
import json
import random
import logging
from tqdm import tqdm

# הגדרת הלוגר
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler()
    ]
)
logger = logging.getLogger(__name__)

def custom_tokenizer(nlp):
    default_tokenizer = Tokenizer(nlp.vocab)
    nlp2 = Hebrew()
    
    LIST_BREAKING_WORDS = [r'—', r'--', r'-', r'\+']
    LIST_AMPERSAND = [r"[\x2D&]"]
    LIST_MORE = [r"״", "\."]

    custom_patterns = (
        LIST_QUOTES +
        LIST_ELLIPSES +
        LIST_BREAKING_WORDS +
        LIST_AMPERSAND +
        LIST_CURRENCY +
        LIST_PUNCT +
        LIST_ICONS +
        LIST_MORE
    )
    
    prefixes = nlp2.Defaults.prefixes + custom_patterns + [r'^(?!וו)ו']
    infixes = nlp2.Defaults.infixes + custom_patterns
    suffixes = nlp2.Defaults.suffixes + custom_patterns

    prefix_regex = spacy.util.compile_prefix_regex(prefixes)
    infix_regex = spacy.util.compile_infix_regex(infixes)
    suffix_regex = spacy.util.compile_suffix_regex(suffixes)

    nlp2.tokenizer.prefix_search = prefix_regex.search
    nlp2.tokenizer.infix_finditer = infix_regex.finditer
    nlp2.tokenizer.suffix_search = suffix_regex.search
    
    return nlp2.tokenizer

def main():
    try:
        nlp = spacy.blank("he")
        nlp.tokenizer = custom_tokenizer(nlp)
        
        test_text = "תומר כהן- הישראלי הבכיר בלינקדין"
        doc = nlp(test_text)
        logger.info(f"Tokenized Text: {[token.text for token in doc]}")
 
 
        if "ner" not in nlp.pipe_names:
            ner = nlp.add_pipe("ner", last=True)
        else:
            ner = nlp.get_pipe("ner")

        if "SINGER" not in ner.labels:
            ner.add_label("SINGER")
        
        ner.add_label("SONG")
        ner.add_label("MISC")
        ner.add_label("ALBUM")
        ner.add_label("GENRE")
        
        
        json_files = [
            'cleaned_new-data.json'
        ]
        
        # טעינת הנתונים
        training_data = []
        for json_file in json_files:
            with open(json_file, 'r', encoding='utf-8') as f:
                data = json.load(f)
                for example_text, example_entities in data:
                    entities = example_entities.get('entities', [])
                    example = Example.from_dict(nlp.make_doc(example_text), {'entities': entities})
                    training_data.append(example)

        logger.info(f"Loaded {len(training_data)} training examples.")

        # ערבוב רנדומלי ראשוני
        random.shuffle(training_data)
        logger.info("Initial shuffle of training data completed.")

        # התחלת אימון
        nlp.begin_training()

        
        patience = 3
        min_delta = 1000
        best_loss = float('inf')
        patience_counter = 0
        best_model_path = "best_model"
        n_iter = 20
        batch_size = 64
        drop_size = 0.35
        iteration_data = {}
        
        logger.info("Starting training...")
        
        for itn in tqdm(range(n_iter), desc="Training Iterations"):
            random.shuffle(training_data)
            losses = {}
            for i in range(0, len(training_data), batch_size):
                batch = training_data[i:i + batch_size]
                nlp.update(batch, drop=drop_size, losses=losses)
            logger.info(f"Iteration {itn}: {losses}")
            iteration_data[itn] = losses.copy()
            
            current_loss = losses.get('ner', float('inf'))
            if current_loss < best_loss - min_delta:
                best_loss = current_loss
                patience_counter = 0
                # שמירת המודל הטוב ביותר
                nlp.to_disk(best_model_path)
                logger.info(f"New best model found at iteration {itn} with loss {current_loss}. Saved to {best_model_path}.")
            else:
                patience_counter += 1
                logger.info(f"No improvement in iteration {itn}. Patience counter: {patience_counter}/{patience}")
            
            if patience_counter >= patience:
                logger.info(f"Early stopping at iteration {itn} due to no improvement.")
                break
        

        model_name = "custom_ner_model30git"
        logger.info(f'Final Model Name: {model_name}')
        
        try:
            with open('iteration_data.json', 'w', encoding='utf-8') as f:
                json.dump(iteration_data, f, ensure_ascii=False, indent=2)
            logger.info("Saved iteration data to JSON file.")
        except Exception as e:
            logger.error(f'Error saving iteration data to JSON file: {e}')
        
        # טעינת המודל הטוב ביותר לפני השמירה עם השם הסופי
        nlp = spacy.load(best_model_path)
        nlp.meta['name'] = 'singer_ner_he'
        nlp.meta['description'] = 'Model for recognizing singer names in Hebrew song titles'
        nlp.meta['author'] = 'nhlocal'
        nlp.meta['email'] = 'nh.local11@gmail.com'
        nlp.meta['license'] = 'MIT'
        nlp.meta['tags'] = ['NER', 'Hebrew', 'Singer', 'Named Entity Recognition', 'Text Classification']
        nlp.to_disk(model_name)
        logger.info(f"Saved final model to {model_name}.")
    
    except Exception as e:
        logger.error(f"An error occurred during training: {e}")

if __name__ == "__main__":
    main()
