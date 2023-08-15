from spacy.lang.he import Hebrew
from spacy.tokenizer import Tokenizer
import spacy

def custom_tokenizer(nlp):
    # Load the default tokenizer
    default_tokenizer = Tokenizer(nlp.vocab)
    nlp2 = Hebrew()
    # Define the custom tokenization rule for "ו" at the beginning of a word using regex
    prefixes = nlp2.Defaults.prefixes + [r'^(?!וו)ו']
    prefix_regex = spacy.util.compile_prefix_regex(prefixes)
    nlp2.tokenizer.prefix_search = prefix_regex.search
    return nlp2.tokenizer

# Load the existing SpaCy model (e.g., 'he_core_news_sm')
nlp = spacy.load("custom_ner_model")

# Set the custom tokenizer as the tokenizer for the pipeline
nlp.tokenizer = custom_tokenizer(nlp)

# Save the model with the updated tokenizer to a new directory
output_dir = "custom_ner_model2"
nlp.to_disk(output_dir)

# Load the model from the updated directory
nlp_updated = spacy.load(output_dir)

# Use the updated model with the custom tokenizer for processing text
text = "והיה זה המקרה"
doc = nlp_updated(text)

# Print the tokens and their start/end character positions
for token in doc:
    print(f"{token.text}, Start: {token.idx}, End: {token.idx + len(token)}")
