import spacy
from spacy.training.example import Example

nlp = spacy.load("custom_ner_model2")
examples = []
data = [
    ("נפתלי כהן בסינגל החדש שלו - אליהו הנביא", {"entities": [(0, 9, "SINGER")]}),
    ("השיר החדש של יהושע כהנוביץ - שמחה וששון", {"entities": [(13, 28, "SINGER")]}),
]


for text, annots in data:
    doc = nlp.make_doc(text)
    examples.append(Example.from_dict(doc, annots))
print(nlp.evaluate(examples))

"""
for key, value in results.items():
    if not value is None and not value == 0.0 and not value == {}:
        print(f"{key:<5}: {value}")
"""
        