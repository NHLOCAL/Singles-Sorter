HEB_CHARS = "אבגדהוזחטיכלמנסעפצקרשתךםןףץ"
JIB_CHARS = "àáâãäåæçèéëìîðñòôö÷øùúêíïóõ"

HEB_TO_JIB_TRANS = str.maketrans(HEB_CHARS, JIB_CHARS)
JIB_TO_HEB_TRANS = str.maketrans(JIB_CHARS, HEB_CHARS)
JIB_CHAR_SET = set(JIB_CHARS)


def _auto_detect_mode(string):
    """Preserve legacy behavior: decide by first convertible character."""
    for letter in string:
        if letter in HEB_CHARS:
            return "jib"
        if letter in JIB_CHARS:
            return "heb"
    return None


def fix_jibrish(string, mode="heb"):
    """
    Convert between garbled Hebrew encoding and regular Hebrew.

    Parameters:
        string (str): Input text.
        mode (str): "heb" to convert gibberish -> Hebrew, "jib" for Hebrew -> gibberish.
                    Any other value enables auto detection.
    """
    if not string:
        return string

    conversion_mode = mode
    if conversion_mode not in ("heb", "jib"):
        conversion_mode = _auto_detect_mode(string)
        if conversion_mode is None:
            return string

    if conversion_mode == "heb":
        return string.translate(JIB_TO_HEB_TRANS)

    return string.translate(HEB_TO_JIB_TRANS)



def check_jibrish(string):
    """
    Check whether the string contains garbled Hebrew characters.

    Parameters:
        string (str): Input text.
    """

    return any(c in JIB_CHAR_SET for c in string)


if __name__ == '__main__':
    string = "àìáåí ìà éãåò & - 3"
    print(fix_jibrish(string))
