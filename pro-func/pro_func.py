# -*- coding: utf-8 -*-
import os
import sys
from os.path import join, getsize
from bidi.algorithm import get_display
   


def pro_scanner(my_file):
    print(my_file)
        

    #os.system('pause')

   
def main():
    dir_path = str(sys.argv[1])
    if (dir_path != "") and (os.path.exists(dir_path)):
        for root, dirs, files in os.walk(dir_path):
            for my_file in files:
                pro_scanner(my_file)

if __name__ == '__main__':
    main()