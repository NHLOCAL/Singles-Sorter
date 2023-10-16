import os
import re
from check_name import artist_from_song, check_exact_name


def test_artist_from_song():
    # Test case 1: Artist name exists in the file name
    assert artist_from_song("אהבת עולם - אברהם פריד.mp3") == "אברהם פריד"
    
    # Test case 2: Artist name doesn't exist in the file name
    assert artist_from_song("תפילה לשלום המדינה.mp3") is None

def test_check_exact_name():
    # Test case 1: Exact match
    assert check_exact_name("מוטי שטיינמץ - שיר", "מוטי שטיינמץ") is True
    
    # Test case 2: Artist name appears at the start of the file name
    assert check_exact_name("#מוטי שטיינמץ$-שיר חדש", "מוטי שטיינמץ") is True
    
    # Test case 3: Artist name appears at the end of the file name
    assert check_exact_name("שיר של (מוטי שטיינמץ)", "מוטי שטיינמץ") is True
    
    # Test case 4: Artist name appears in the middle of the file name
    assert check_exact_name("אהבת מוטי שטיינמץ עולם", "מוטי שטיינמץ") is True
    
    # Test case 5: Artist name does not match
    assert check_exact_name("שיר נוסף - למוטי שטיינמץ", "מוטי שטיינמץ") is False
    
    assert check_exact_name("שיר נוסף - מומוטי שטיינמץ חדש", "מוטי שטיינמץ") is False
    
    assert check_exact_name("למוטי שטיינמץ", "מוטי שטיינמץ") is False
    
    assert check_exact_name("שיר נוסף - מוטי שטיינמץל", "מוטי שטיינמץ") is False

    assert check_exact_name("שיר נוסף - ומוטי שטיינמץ", "מוטי שטיינמץ") is True

    assert check_exact_name("שיר נוסף - ומוטי שטיינמץ שר", "מוטי שטיינמץ") is True
    
    assert check_exact_name("ומוטי שטיינמץ שר", "מוטי שטיינמץ") is True


if __name__ == "__main__":
    test_check_exact_name()