import requests

def check_for_update(repo_owner, repo_name, current_version):
    url = f"https://api.github.com/repos/{repo_owner}/{repo_name}/tags"
    print(url)
    response = requests.get(url)
    if response.status_code == 200:
        data = response.json()
        print(data)
        if data:
            latest_version = data[0]['name']
            
            if latest_version != current_version:
                print(f"New version available: {latest_version}")
                return latest_version
            else:
                print("You are using the latest version.")
                return None
        else:
            print("No tags found in the repository.")
            return None
    else:
        print("Failed to fetch tags information.")
        return None

# פרטים למאגר שלך
repo_owner = "NHLOCAL"  # שם המשתמש בגיטהאב
repo_name = "Singles-Sorter"  # שם המאגר
current_version = "v1.0.0"  # הגרסה הנוכחית שלך

latest_version = check_for_update(repo_owner, repo_name, current_version)
if latest_version:
    print(f"New version tag: {latest_version}")
else:
    print("No update needed.")
