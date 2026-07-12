"""Short contextual tips displayed without interrupting the primary flow."""

from datetime import date

TIPS = (
    "מומלץ ליצור גיבוי לפני העברה או תיקון של אוסף חשוב.",
    "רשימת זמרים אישית עוזרת לזהות כינויים וכתיבים מיוחדים.",
    "אפשר לעצור פעולה ארוכה בבטחה מחלון ההתקדמות.",
    "מצב העתקה משאיר את הקבצים המקוריים במקומם.",
    "אפשר לתקן ג׳יבריש ושמות מיותרים גם בלי לבצע מיון.",
)


def tip_for_index(index: int) -> str:
    return TIPS[index % len(TIPS)]


def daily_tip(today: date | None = None) -> str:
    return tip_for_index((today or date.today()).toordinal())
