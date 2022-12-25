from pro_func import artist_from_song, check_artist
import sys


def main():
    try:
        file_path = str(sys.argv[1])
        target_path = str(sys.argv[2])
        arg_list = []
        for arg in sys.argv:
            arg = str(arg)
            arg_list.append(arg)
        arg_list = arg_list[3::]

        arg_list.sort(key=sort_arg)
        print(arg_list)
        '''
        artist = artist_from_song(arg_list)
        checking = check_artist(artist)
        if checking:
            print(artist)
        else:
            print(False)
        '''
    except Exception as e:
        print("הכנס נתיב שיר לקבלת שם האמן שלו")


def sort_arg(arg):
    # ארגומנט יצירת תיקית סינגלים
    if arg == "s-f:yes" or arg == "s-f:no": return 1
    # ארגומנט יצירת תיקיות א' ב'
    elif arg == "ab-f:yes" or arg == "ab-f:no": return 2
    # ארגומנט העתקה או העברה
    elif arg == "copy" or arg == "move": return 3
    # ארגומנט העברה לתיקיה קיימת בלבד
    elif arg == "o-exist:yes" or arg == "o-exist:no": return 4
    # ארגומנט מיון תיקיה ראשית בלבד
    elif arg == "o-main:yes" or arg == "o-main:no": return 5
    else: return False


if __name__ == '__main__':
    main()
    
    
