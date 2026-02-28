import logging
import os
import pickle
from importlib.resources import as_file, files
from pathlib import Path


class AIModels:
    """Load and expose optional AI models for artist detection."""

    def __init__(self, logger=None):
        if logger:
            self.logger = logger
        else:
            self.logger = logging.getLogger('AIModels')
            self.logger.setLevel(logging.INFO)

            if not self.logger.handlers:
                console_handler = logging.StreamHandler()
                console_handler.setLevel(logging.INFO)
                formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
                console_handler.setFormatter(formatter)
                self.logger.addHandler(console_handler)

        self.nlp = None
        self.sklearn_model = None
        self.available = False

        self._spacy_load = None
        self._load_models()
        self.available = bool(self.nlp or self.sklearn_model)

    def _load_models(self):
        if not self._load_optional_dependencies():
            self.logger.info(
                "AI features are disabled because optional dependencies are missing. "
                "Install with: pip install singlesorter[ai]"
            )
            return

        if self._load_from_packaged_resources():
            return

        self._load_from_filesystem_fallbacks()

    def _load_optional_dependencies(self):
        try:
            from spacy import load as spacy_load
        except ImportError:
            return False

        try:
            import sklearn.feature_extraction.text  # noqa: F401
            import sklearn.pipeline  # noqa: F401
        except ImportError:
            return False

        self._spacy_load = spacy_load
        return True

    def _load_from_packaged_resources(self):
        try:
            package_root = files('singlesorter').joinpath('models')
            clf_resource = package_root.joinpath('music_entity_clf', 'music_entity_clf.pkl')
            ner_resource = package_root.joinpath('singer_ner_he')
            ner_meta_resource = ner_resource.joinpath('meta.json')

            if not (clf_resource.is_file() and ner_meta_resource.is_file()):
                return False

            with as_file(ner_resource) as ner_path, as_file(clf_resource) as clf_path:
                return self._load_model_files(Path(ner_path), Path(clf_path))
        except Exception as exc:
            self.logger.debug(f"Packaged model lookup failed: {exc}")
            return False

    def _load_from_filesystem_fallbacks(self):
        for root in self._iter_fallback_roots():
            clf_path = root / 'music_entity_clf' / 'music_entity_clf.pkl'
            ner_path = root / 'singer_ner_he'
            ner_meta_path = ner_path / 'meta.json'
            if clf_path.is_file() and ner_meta_path.is_file():
                if self._load_model_files(ner_path, clf_path):
                    return

        self.logger.info("AI model files were not found. Continuing with non-AI detection only.")

    def _iter_fallback_roots(self):
        seen = set()
        env_model_dir = os.getenv('SINGLESORTER_MODEL_DIR')
        candidates = []

        if env_model_dir:
            candidates.append(Path(env_model_dir).expanduser())

        candidates.extend(
            [
                Path(__file__).resolve().parent / 'models',
                Path.cwd() / 'models',
                Path.cwd() / 'src' / 'models' / 'models',
            ]
        )

        for candidate in candidates:
            resolved = candidate.resolve()
            if resolved not in seen:
                seen.add(resolved)
                yield resolved

    def _load_model_files(self, ner_path, clf_path):
        try:
            self.nlp = self._spacy_load(str(ner_path))
            self.logger.debug(f"Loaded NER model from: {ner_path}")
        except Exception as exc:
            self.nlp = None
            self.logger.error(f"Failed to load NER model from {ner_path}: {exc}")

        try:
            with open(clf_path, 'rb') as model_file:
                self.sklearn_model = pickle.load(model_file)
            self.logger.debug(f"Loaded sklearn model from: {clf_path}")
        except Exception as exc:
            self.sklearn_model = None
            self.logger.error(f"Failed to load sklearn model from {clf_path}: {exc}")

        return bool(self.nlp or self.sklearn_model)

    def process_with_ner(self, text):
        if not self.nlp:
            return []

        doc = self.nlp(text)
        potential_artists = [ent.text for ent in doc.ents if ent.label_ == 'SINGER']

        verified_artists = []
        for artist in potential_artists:
            self.logger.debug(f"NER model identified potential artist: {artist}")
            if self.verify_artist_with_sklearn(artist):
                verified_artists.append(artist)

        return verified_artists

    def verify_artist_with_sklearn(self, artist_name):
        if not self.sklearn_model:
            self.logger.warning('sklearn model not available for verification')
            return True

        input_data = [artist_name]

        try:
            prediction = self.sklearn_model.predict(input_data)
            probabilities = self.sklearn_model.predict_proba(input_data)[0]

            predicted_class = prediction[0]
            class_probability = probabilities[predicted_class]

            class_names = ['ARTIST', 'ALBUM', 'SONG', 'RANDOM']

            self.logger.debug(
                f"sklearn model prediction for '{artist_name}': "
                f"class={class_names[predicted_class]}, "
                f"probability={class_probability:.2f}"
            )

            is_artist = predicted_class == 0

            if is_artist:
                self.logger.debug(f"'{artist_name}' verified as an artist")
            else:
                self.logger.debug(
                    f"'{artist_name}' not verified as an artist. "
                    f"Predicted as: {class_names[predicted_class]}"
                )

            return is_artist

        except Exception as exc:
            self.logger.error(f"Error during sklearn prediction for '{artist_name}': {exc}")
            return True
