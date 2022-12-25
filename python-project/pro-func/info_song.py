import pro_func
import sys



def main():
    try:
        filepath = str(sys.argv[1])
        artist = pro_func.artist_from_song(filepath)
        checking = pro_func.check_artist(artist)
        if checking:
            print(artist)
        else:
            print(False)
    except:
         print("הכנס נתיב שיר  לקבלת שם האמן שלו")
if __name__ == '__main__':
    main()