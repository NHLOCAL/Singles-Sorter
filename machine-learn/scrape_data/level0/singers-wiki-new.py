import mwclient

def get_category_members(site, category_name, max_level=1, level=0):
    if level > max_level:
        return []
    category = site.pages[category_name]
    members = []
    for member in category.members(namespace=0):
        members.append(member.name)
    for subcategory in category.members(namespace=14):
        members += get_category_members(site, subcategory.name, max_level, level + 1)
    return members

# התחברות לאתר ויקיפדיה בעברית
site = mwclient.Site('www.hamichlol.org.il')

# שם הקטגוריה
category_name = 'קטגוריה:זמרים חסידיים'

# שליפת שמות הזמרים
members = get_category_members(site, category_name, max_level=2)

# הדפסת השמות
for member in members:
    print(member)

# עתה אתה חמוש בכל השמות! זה הזמן להתכונן למרתון של "מי מכיר את הזמר" 🎤🎉
