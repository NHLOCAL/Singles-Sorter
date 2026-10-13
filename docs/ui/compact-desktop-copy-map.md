# Compact desktop interface: wording and verification

Released reference: `src/core/main.py` at `78d001a` (14.0). The current host remains read-only; profile schema, engine rules, AI availability and IPC contracts are unchanged.

## Original → current

- `מסדר הסינגלים` → unchanged.
- `תיקית מקור`, `תיקית יעד`, `בחר תיקיה` → restored exactly.
- `מיין שירים` → `מיין שירים (תצוגה מקדימה)`. Only the parenthetical is added because this host can plan, not apply.
- `תקן שמות` → `תקן שמות (תצוגה מקדימה)`. Same capability restriction.
- `העבר קבצים` / `העתק קבצים` → restored dynamic switch label, with the same checked boolean.
- `סרוק עץ תיקיות` / `סרוק תיקיה ראשית בלבד` → restored dynamic switch label.
- `צור תיקיות סינגלים`, `שימוש בתיקיות קיימות`, `מיון אלפביתי` → exact released labels.
- `הגדרות מתקדמות`, `מיון דואטים`, `לזמר הראשון בשם השיר`, `לכל הזמרים המופיעים בשיר` → restored exactly.
- Save tooltip `שמור הגדרות מותאמות אישית` → visible compact command `שמור הגדרות`; existing explicit persistence unchanged.
- Released folder and setting tooltip descriptions → contextual help badges; wording retained with the target-folder clarification that it is required only for sorting.
- First-five-save and immediate-duet semantics → contextual help; unchanged behavior.
- `הוספת זמרים`, `ערוך רשימה`, `יבא קובץ CSV`, `יצא לקובץ CSV`, editor fields `שם תיקיה` / `שם זמר` → restored exactly. Original editor `אישור` → `שמור רשימה` because the current editor explicitly saves and remains open; `ביטול` and `הוסף שורה` remain unchanged.
- Released action descriptions for sorting and fixing → optional action help badges, not static prose.
- New host-only AI/profile/list/result controls retain their existing functional labels. No released capability is falsely implied.

Development/preview banners, paragraph explanations, footer privacy prose and automatic first-run help are removed. Optional help stays in the menu. Real errors and result blockers remain visible.

## Design and acceptance

52px blue app header, paired working panes, Segoe UI first on Windows, 14px base text, 20px title, 16px section headings. Normal desktop keeps heading and pagination outside one scrolling result body. Large text, short windows and narrow screens reflow instead of clipping controls. Touch controls are at least 44px.

Reference: Microsoft [Windows typography](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/typography) and [command bars](https://learn.microsoft.com/en-ca/windows/apps/design/controls/command-bar). This remains a Tauri/WebView interface, not a claim to use native WinUI controls.

Local checks: TypeScript/Vite build and 40 unit tests pass. Renderer acceptance is implemented but local Chromium cannot start under this executor's socket restrictions, including an escalated attempt. No rendered screenshot or actual Segoe Hebrew glyph result is claimed. Windows acceptance must establish actual 1000×650 / 1024×768 CSS viewports and save screenshots and measured DPR/font styles. Renderer DPR 1.25/1.5 checks are explicitly distinct from Windows OS scaling. `document.fonts.check` does not prove which font rendered Hebrew; platform glyph evidence requires CDP or equivalent.
