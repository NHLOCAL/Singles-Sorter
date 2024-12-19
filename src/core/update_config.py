# -*- coding: utf-8 -*-
import os
import requests

def check_for_update(current_version):
    url = "https://api.github.com/repos/NHLOCAL/Singles-Sorter/releases/latest"
    
    try:
        response = requests.get(url)
        response.raise_for_status()
        latest_release = response.json()
        
        is_prerelease = latest_release['prerelease']
        latest_version = latest_release['tag_name'].strip()
        
        if latest_version > f"v{current_version}" and not is_prerelease:
            return (
                latest_version,
                latest_release.get('body', '')
            )
        else:
            return False
        
    except requests.RequestException as e:
        print(f"שגיאה בקבלת נתוני גרסה: {e}")
        return False

def check_ai_model_update():
    """
    Check for the latest AI model version on GitHub and compare it with the local version.

    Returns:
        tuple: (is_update_available, latest_version, release_notes) or (False, None, None) if no update is available.
    """
    github_repo = "NHLOCAL/SingNER"
    url = f"https://api.github.com/repos/{github_repo}/releases/latest"
    local_version_file = "models/version"

    try:
        # Fetch the latest release information from GitHub
        response = requests.get(url)
        response.raise_for_status()
        latest_release = response.json()

        latest_version = latest_release['tag_name'].lstrip('v')
        release_notes = latest_release.get('body', '')

        # Read the current local version
        if os.path.isfile(local_version_file):
            with open(local_version_file, "r", encoding="utf-8") as file:
                local_version = file.read().strip()
        else:
            local_version = "0.0.0"

        # Compare versions
        if latest_version > local_version:
            return True, latest_version, release_notes
        else:
            return False, None, None

    except requests.RequestException as e:
        print(f"Error fetching AI model release data: {e}")
        return False, None, None

def download_and_update_ai_model(latest_version):
    """
    Download the latest AI model and update the local version file.

    Args:
        latest_version (str): The latest version tag.

    Returns:
        bool: True if the update was successful, False otherwise.
    """
    download_url = f"https://github.com/NHLOCAL/SingNER/releases/download/v{latest_version}/SingNER-models-v{latest_version}.zip"
    model_directory = "models"
    zip_file_path = f"{model_directory}/SingNER-models-v{latest_version}.zip"

    try:
        # Download the zip file containing the new model
        response = requests.get(download_url, stream=True)
        response.raise_for_status()

        with open(zip_file_path, "wb") as file:
            for chunk in response.iter_content(chunk_size=8192):
                file.write(chunk)

        print(f"Downloaded AI model version {latest_version}.")

        # Extract the downloaded zip file
        import zipfile
        with zipfile.ZipFile(zip_file_path, "r") as zip_ref:
            zip_ref.extractall(model_directory)

        print("AI model extracted successfully.")

        # Update the version file
        with open(f"{model_directory}/version", "w", encoding="utf-8") as file:
            file.write(latest_version)

        # Clean up the zip file
        os.remove(zip_file_path)

        return True

    except Exception as e:
        print(f"Error updating AI model: {e}")
        return False

if __name__ == '__main__':
    from singles_sorter_v5 import __VERSION__
    current_version = __VERSION__

    update_info = check_for_update(current_version)
    if update_info:
        new_version, release_notes = update_info
        print(f"גרסה חדשה זמינה: {new_version} 🚀")
        print(f"הערות שחרור:\n{release_notes}")
    else:
        print("אין גרסה חדשה זמינה. 🙁")

    # Check for AI model updates
    is_ai_update_available, ai_latest_version, ai_release_notes = check_ai_model_update()

    if is_ai_update_available:
        print(f"New AI model version {ai_latest_version} is available.")
        print("Release Notes:")
        print(ai_release_notes)

        user_input = input("Would you like to update to the latest AI model? (yes/no): ").strip().lower()

        if user_input == "yes":
            success = download_and_update_ai_model(ai_latest_version)
            if success:
                print(f"AI model successfully updated to version {ai_latest_version}.")
            else:
                print("Failed to update AI model.")
        else:
            print("AI model update canceled.")
    else:
        print("Your AI model is up-to-date.")
