import re
import requests
from bs4 import BeautifulSoup

def scrape_singer_names(category):
    url = "https://he.wikipedia.org/wiki/" + "קטגוריה:" + category + "/"
    response = requests.get(url)
    soup = BeautifulSoup(response.content, "html.parser")

    singer_names = []
    for link in soup.find_all("div", class_="mw-category-group"):
        for item in link.find_all("a"):
            singer_names.append(item.get("title"))

    singer_names = [name.replace(category, "") for name in singer_names]
    return singer_names


# הרץ את הפונקציה

category_list = [
"זמרים ישראלים",
"זמרים חסידיים",
"זמרים חסידיים אמריקאים‏",
"זמרים חסידיים ישראלים‏",
'מוזיקאים וזמרים חסידי חב"ד',
"זמרים חסידיים צרפתים‏",
"זמרי מוזיקה מזרחית ישראלים",
"זמרים-יוצרים ישראלים",
"משתתפי פסטיבל הזמר החסידי",
"משתתפי פסטיבל הזמר המזרחי",
'חזנים',
]


all_singers_set = set()

for category in category_list:
    israeli_singers = [singer for singer in scrape_singer_names(category) if not "קטגוריה" in singer]
    all_singers_set.update(israeli_singers)

# הסרת תוכן לא רצוי מהמחרוזת
pattern = r'\s*\([^)]*\)'
all_singers_set = [re.sub(pattern, '', item) for item in all_singers_set]

for singer in all_singers_set:
    print(singer)
