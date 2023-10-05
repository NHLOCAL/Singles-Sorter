def jibrish_to_hebrew(string, mode="heb"):
    """
פונקציה שממירה מחרוזת משובשת לעברית
ניתן להכניס מחרוזת בעברית על מנת להמירה לג'יבריש ולהפך

פרמטרים:
פרמטר 1 = מחרוזת בעברית משובשת
פרמטר 2 (אופציונלי) = הגדרת מצב המרה לעברית או ג'יבריש - מקבל "heb" או "jib"
    """

    heb_to_jib = {'א': 'à', 'ב': 'á', 'ג': 'â', 'ד': 'ã', 'ה': 'ä', 'ו': 'å', 'ז': 'æ', 'ח': 'ç', 'ט': 'è', 'י': 'é', 'כ': 'ë', 'ל': 'ì', 'מ': 'î','נ': 'ð', 'ס': 'ñ', 'ע': 'ò', 'פ': 'ô', 'צ': 'ö', 'ק': '÷', 'ר': 'ø', 'ש': 'ù', 'ת': 'ú', 'ך': 'ê', 'ם': 'í', 'ן': 'ï', 'ף': 'ó', 'ץ': 'õ'}
    jib_to_heb = {'à': 'א', 'á': 'ב', 'â': 'ג', 'ã': 'ד', 'ä': 'ה', 'å': 'ו', 'æ': 'ז', 'ç': 'ח', 'è': 'ט', 'é': 'י', 'ë': 'כ', 'ì': 'ל', 'î': 'מ', 'ð': 'נ', 'ñ': 'ס', 'ò': 'ע', 'ô': 'פ', 'ö': 'צ', '÷': 'ק', 'ø': 'ר', 'ù': 'ש', 'ú': 'ת', 'ê': 'ך', 'í': 'ם', 'ï': 'ן', 'ó': 'ף', 'õ': 'ץ'}

    new_string = ""
    
    for letter in string:
        if mode == "heb":
            if letter in jib_to_heb:
                new_letter = jib_to_heb[letter]
                new_string += new_letter
            else:
                new_string += letter
        elif mode == "jib":
            if letter in heb_to_jib:
                new_letter = heb_to_jib[letter]
                new_string += new_letter
            else:
                new_string += letter
        else:
            if letter in "אבגדהוזחטיכלמנסעפצקרשתךםןףץ":
                mode = "jib"
                if letter in heb_to_jib:
                    new_letter = heb_to_jib[letter]
                    new_string += new_letter
                else:
                    new_string += letter
            elif letter in "àáâãäåæçèéëìîðñòôö÷øùúêíïóõ":
                mode = "heb"
                if letter in jib_to_heb:
                    new_letter = jib_to_heb[letter]
                    new_string += new_letter
                else:
                    new_string += letter
            else:
                new_string += letter

    return new_string


def main():
    string = "àìáåí ìà éãåò & - 3"
    print(jibrish_to_hebrew(string))


if __name__ == '__main__':
    main()
