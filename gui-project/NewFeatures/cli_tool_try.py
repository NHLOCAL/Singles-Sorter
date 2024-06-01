import os
import argparse

def clean_names(dir_path):
    # Function to clean file names
    pass

def scan_dir(dir_path, target_dir=None, copy_mode=False, abc_sort=False, exist_only=False, singles_folder=True, main_folder_only=False, progress_callback=None):
    """
    Main function of the program. Scans the specified directory and creates a list of files for copying.
    At the end of the process, it copies them if a target directory parameter is provided.
    
    Parameters:
        dir_path = Directory path to scan
        target_dir = Target directory path for transfer (optional)
        copy_mode = Enable copy mode (default is move)
        abc_sort = Sort folders alphabetically
        exist_only = Transfer to existing folders only
        singles_folder = Create an internal "singles" folder
        main_folder_only = Sort only the main folder
        Defined by True or False.
    
    Result:
        Prints the list of artists that appear in the song metadata and copies them to the target.
    """
    print(f"Scanning directory: {dir_path}")
    print(f"Target directory: {target_dir}")
    print(f"Copy mode: {copy_mode}")
    print(f"Alphabetical sort: {abc_sort}")
    print(f"Exist only: {exist_only}")
    print(f"Singles folder: {singles_folder}")
    print(f"Main folder only: {main_folder_only}")

def main():
    parser = argparse.ArgumentParser(description="Single Organizer 13.0 - Scan and organize music files into folders by artist using advanced automation.")
    parser.add_argument('dir_path', help="Path to the source directory")
    parser.add_argument('target_dir', help="Path to the target directory", nargs='?', default=None)

    parser.add_argument('--copy_mode', help="Enable copy mode (default is move mode)", action='store_true')
    parser.add_argument('--abc_sort', help="Sort folders alphabetically", action='store_true')
    parser.add_argument('--exist_only', help="Transfer to existing folders only", action='store_true')
    parser.add_argument('--no_singles_folder', help="Do not create an internal 'singles' folder", action='store_false', dest='singles_folder', default=True)
    parser.add_argument('--main_folder_only', help="Sort only the main folder (default: False)", action='store_true')

    args = parser.parse_args()

    try:
        dir_path = os.path.join(args.dir_path) # Source directory path
        target_dir = os.path.join(args.target_dir) if args.target_dir else None # Target directory path (optional)
        copy_mode = args.copy_mode  # Set copy mode
        abc_sort = args.abc_sort  # Alphabetical sorting
        exist_only = args.exist_only  # Transfer to existing folders only        
        singles_folder = args.singles_folder  # Internal singles folder
        main_folder_only = args.main_folder_only  # Main folder only

        # Run the clean names function
        clean_names(dir_path)
    
        # Run the scan directory function with all parameters
        scan_dir(dir_path, target_dir, copy_mode, abc_sort, exist_only, singles_folder, main_folder_only)
    except Exception as e:
        print("Error: {}".format(e))

if __name__ == "__main__":
    main()
