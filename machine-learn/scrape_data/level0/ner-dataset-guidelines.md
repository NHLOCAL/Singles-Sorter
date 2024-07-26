# Guidelines for Creating NER Dataset for Singer Names in Song Titles

1. **Data Structure**:
   - Each record: Song title + Singer name tag (if present)
   - Use BIO or BILOU tagging format

2. **Data Distribution**:
   - 60% (600) songs with singer names
   - 40% (400) songs without singer names

3. **Singer Name Distribution**:
   - 75 popular singers (25%)
   - 225 less known singers (75%)

4. **Appearance Frequency**:
   - Popular singers: 10-20 appearances each
   - Less known singers: 1-5 appearances each

5. **Music Genre Diversity**:
   - Include singers from various genres

6. **Geographic and Linguistic Diversity**:
   - Include names from different countries and cultures
   - Use names in various languages and transliterations

7. **Edge Cases**:
   - Complex names (e.g., "John Lennon")
   - Stage names (e.g., "Madonna", "Sting")
   - Names that are also common words

8. **Name Variations**:
   - Include different versions of the same name

9. **Gender Balance**:
   - Ensure balanced representation of male and female artists

10. **Name Position in Title**:
    - Vary the position of the singer's name in the song title

11. **Quality Control**:
    - Manual verification of tagging accuracy
    - Review dataset statistics

12. **Data Augmentation**:
    - Consider techniques like word order changes or word replacements

13. **Dataset Split**:
    - Training set (70%)
    - Validation set (15%)
    - Test set (15%)
    - Ensure balanced representation across all sets
