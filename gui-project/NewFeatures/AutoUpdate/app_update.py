import flet as ft
import requests
import os

def get_current_version():
    try:
        with open('version.txt', 'r') as f:
            return f.read().strip()
    except FileNotFoundError:
        return "0.0.0"

def get_latest_version_info():
    response = requests.get('https://nhlocal.github.io/Singles-Sorter/versions.data/new-ver-exist')
    return response.text.strip()

def update_available(current_version, latest_version):
    return latest_version > current_version

def download_update(url, local_filename):
    response = requests.get(url, stream=True)
    with open(local_filename, 'wb') as f:
        for chunk in response.iter_content(chunk_size=8192):
            f.write(chunk)

def install_update(filename):
    os.system(f'start /wait {filename}')
    with open('version.txt', 'w') as f:
        f.write(latest_version)

def main(page: ft.Page):
    page.title = "My App"
    page.add(ft.Text("Checking for updates..."))

    current_version = '12.9.0' #get_current_version()
    latest_version = '12.9.0' #get_latest_version_info()

    if update_available(current_version, latest_version):
        update_text = f"New version {latest_version} available! Current version is {current_version}."
        update_button = ft.ElevatedButton(
            "Download and Install",
            on_click=lambda e: update_action(page, f'https://github.com/NHLOCAL/Singles-Sorter/releases/download/v12.8/Singles-Sorter-{latest_version}.exe', latest_version)
        )
        page.add(ft.Text(update_text), update_button)
    else:
        page.add(ft.Text("Your software is up-to-date 😊"))

def update_action(page: ft.Page, url, latest_version):
    page.add(ft.Text("Downloading update..."))
    download_update(url, 'update.exe')
    page.add(ft.Text("Installing update..."))
    install_update('update.exe')
    with open('version.txt', 'w') as f:
        f.write(latest_version)
    page.add(ft.Text("Update complete! Please restart the application."))

ft.app(target=main)
