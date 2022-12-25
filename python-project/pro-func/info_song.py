from pro_func import artist_from_song, check_artist
import sys


def main():
    try:
        filepath = str(sys.argv[1])
        artist = artist_from_song(filepath)
        checking = check_artist(artist)
        if checking:
            print(artist)
        else:
            print(False)
    except:
        print("הכנס נתיב שיר לקבלת שם האמן שלו")


if __name__ == '__main__':
    main()
