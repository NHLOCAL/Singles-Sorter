@echo off
chcp 1255

robocopy c:\Users\me\Documents\GitHub\Singles-Sorter-ml\machine-learn\scrape_data\level0\Gemini-synthetic-v2\best_model c:\Users\me\Documents\GitHub\Singles-Sorter-ml\machine-learn\models\custom_ner_model30-3git /MIR /NFL /NDL /NJH /NJS /NC /NS /NP && echo Copy completed successfully!