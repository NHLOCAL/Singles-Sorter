import logging
from spacy import load
import pickle
import sklearn.pipeline
import sklearn.feature_extraction.text

class AIModels:
    def __init__(self, logger=None):
        if logger:
            self.logger = logger
        else:
            self.logger = logging.getLogger('AIModels')
            self.logger.setLevel(logging.INFO)
            
            # Add a console handler if there isn't one already
            if not self.logger.handlers:
                console_handler = logging.StreamHandler()
                console_handler.setLevel(logging.INFO)
                formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
                console_handler.setFormatter(formatter)
                self.logger.addHandler(console_handler)
        
        self.nlp = None
        self.sklearn_model = None
        self.load_models()

    def load_models(self):
        # Load the NER model
        try:
            model_name = 'models/singer_ner_he'
            self.nlp = load(model_name)
            self.logger.debug(f"Loaded NER model: {model_name}")
        except Exception as e:
            self.logger.error(f"Failed to load NER model: {str(e)}")

        # Load the sklearn model
        try:
            model_path = 'models/music_classifier.pkl'
            with open(model_path, 'rb') as model_file:
                self.sklearn_model = pickle.load(model_file)
            self.logger.debug("Loaded sklearn model successfully")
        except Exception as e:
            self.logger.error(f"Failed to load sklearn model: {str(e)}")

    def process_with_ner(self, text):
        if not self.nlp:
            return []
        
        doc = self.nlp(text)
        potential_artists = [ent.text for ent in doc.ents if ent.label_ == "SINGER"]
        
        verified_artists = []
        for artist in potential_artists:
            self.logger.debug(f"NER model identified potential artist: {artist}")
            if self.verify_artist_with_sklearn(artist):
                verified_artists.append(artist)
        
        return verified_artists

    def verify_artist_with_sklearn(self, artist_name):
        if not self.sklearn_model:
            self.logger.warning("sklearn model not available for verification")
            return True  # Assume it's an artist if model is not available

        # Prepare the input for the model
        input_data = [artist_name]

        try:
            # Make prediction
            prediction = self.sklearn_model.predict(input_data)
            probabilities = self.sklearn_model.predict_proba(input_data)[0]

            # Get the predicted class and its probability
            predicted_class = prediction[0]
            class_probability = probabilities[predicted_class]

            # Define class names for logging
            class_names = ["ARTIST", "ALBUM", "SONG", "RANDOM"]

            # Log the prediction details
            self.logger.debug(f"sklearn model prediction for '{artist_name}': "
                            f"class={class_names[predicted_class]}, "
                            f"probability={class_probability:.2f}")

            # Check if the predicted class is "אמן" (0)
            is_artist = predicted_class == 0

            if is_artist:
                self.logger.debug(f"'{artist_name}' verified as an artist")
            else:
                self.logger.debug(f"'{artist_name}' not verified as an artist. "
                                f"Predicted as: {class_names[predicted_class]}")

            return is_artist

        except Exception as e:
            self.logger.error(f"Error during sklearn prediction for '{artist_name}': {str(e)}")
            return True  # Assume it's an artist if prediction fails
