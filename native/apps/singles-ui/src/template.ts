export const appTemplate = `
<header class="topbar">
  <div class="topbar-start">
    <div class="brand">
      <img src="/icon.png" alt="" width="32" height="32">
      <div class="brand-text">
        <h1>מסדר הסינגלים</h1>
        <span class="brand-tag">ארגון חכם של ספריית המוזיקה</span>
      </div>
    </div>
  </div>
  <div class="topbar-center">
    <nav class="mode-tabs" role="group" aria-label="מצב עבודה ראשי">
      <button type="button" id="tab-mode-sort" class="mode-tab active" aria-pressed="true">
        <span class="tab-label">מיון שירים</span>
      </button>
      <button type="button" id="tab-mode-fix" class="mode-tab" aria-pressed="false">
        <span class="tab-label">תיקון שמות</span>
      </button>
    </nav>
  </div>
  <div class="topbar-end">
    <button id="open-settings" class="icon-button" aria-label="הגדרות" aria-haspopup="dialog"></button>
    <button id="menu" class="icon-button" aria-label="עזרה ומידע" aria-haspopup="dialog">⋮</button>
  </div>
</header>
<main>
  <div id="connection" class="notice" role="status" hidden></div>
  <div id="recovery-notice" class="notice" role="status" hidden>
    <span id="recovery-notice-text"></span>
    <button id="open-recovery" class="secondary" hidden>בדוק את הביצוע הקודם</button>
  </div>
  <div id="error" class="error" role="alert" hidden></div>

  <div class="workspace" id="setup-screen">
    <div class="workspace-main">
      <section class="stations-deck" aria-label="בחירת תיקיות לעבודה">
        <div class="deck-header">
          <div class="deck-title-group">
            <h2 id="folders-heading">בחירת תיקיות</h2>
            <p class="deck-subtitle" id="mode-deck-description">בחר תיקיית מקור לבדיקה ותיקיית יעד לארגון השירים לפי אמנים.</p>
          </div>
        </div>

        <div class="stations-grid">
          <div class="station-card source-station" id="source-station">
            <div class="station-header">
              <div class="field-label">
                <span class="station-badge">1</span>
                <label for="source">תיקיית מקור</label>
                <button class="help-badge" aria-label="עזרה: תיקית מקור" data-help-text="בחירת תיקיה המכילה את המוזיקה שברצונך לסדר.">?</button>
              </div>
            </div>
            <div class="folder-row">
              <input id="source" dir="ltr" autocomplete="off" spellcheck="false" placeholder="C:\\Music\\סינגלים">
              <button id="source-picker" class="secondary">בחר תיקיה</button>
            </div>
            <p class="station-hint">השירים ייסרקו וינותחו מקומית ללא שינוי עד לאישורך.</p>
          </div>

          <div class="station-flow-indicator" id="station-flow" aria-hidden="true">
            <span class="flow-arrow">←</span>
          </div>

          <div class="station-card target-station" id="target-station">
            <div class="station-header">
              <div class="field-label">
                <span class="station-badge">2</span>
                <label for="target">תיקיית יעד</label>
                <button class="help-badge" aria-label="עזרה: תיקית יעד" data-help-text="בחירת תיקית יעד אליה יוכנסו תיקיות המוזיקה שיווצרו. נדרשת למיון בלבד.">?</button>
              </div>
            </div>
            <div class="folder-row">
              <input id="target" dir="ltr" autocomplete="off" spellcheck="false" placeholder="D:\\המוזיקה שלי">
              <button id="target-picker" class="secondary">בחר תיקיה</button>
            </div>
            <p class="station-hint">התיקיות החדשות והשירים הממוינים ייווצרו במיקום זה.</p>
          </div>

        </div>

        <div class="actions-dock">
          <div class="action-command" id="sort-command-wrap">
            <button id="sort" class="primary action-launch">מיין שירים</button>
            <button class="help-badge" aria-label="עזרה: מיין שירים" data-help-text="מיון מתקדם של הסינגלים שלך בתיקיות לפי אמנים.">?</button>
          </div>
          <div class="action-command" id="fix-command-wrap" hidden>
            <button id="fix" class="primary action-launch">תקן שמות</button>
            <button class="help-badge" aria-label="עזרה: תקן שמות" data-help-text="תיקון ג׳יבריש במאפייני הקובץ והסרת תוכן מיותר בשמות הקבצים.">?</button>
          </div>
          <button id="reopen-results" class="text-button" hidden>הצג תוצאות</button>
        </div>
      </section>
    </div>

    <aside class="options card" aria-labelledby="options-heading">
      <div class="options-header">
        <div class="section-title">
          <div class="field-label">
            <h2 id="options-heading">אפשרויות</h2>
            <button class="help-badge" aria-label="עזרה: אפשרויות מיון" data-help-text="חמש האפשרויות נשמרות בלחיצה על שמור הגדרות; מצב דואטים נשמר מיד בהגדרות מתקדמות.">?</button>
          </div>
          <span id="settings-label" class="quiet-label">ההגדרות שלך</span>
        </div>
      </div>

      <div class="options-groups">
        <div class="option-group">
          <span class="group-title">טיפול בקבצים</span>
          <fieldset>
            <legend class="visually-hidden">בסיסי</legend>
            <div class="option-row">
              <label class="option">
                <input id="copy_mode" type="checkbox" role="switch">
                <span id="copy_mode-label">העתק קבצים</span>
              </label>
              <button class="help-badge" aria-label="עזרה: העתק קבצים" data-help-text="קבע אם להעתיק או להעביר את הקבצים לתיקיית היעד">?</button>
            </div>
            <div class="option-row">
              <label class="option">
                <input id="main_folder_only" type="checkbox" role="switch">
                <span id="main_folder_only-label">סרוק רק תיקייה ראשית</span>
              </label>
              <button class="help-badge" aria-label="עזרה: סרוק רק תיקייה ראשית" data-help-text="בחר אם לסרוק את כל תיקיות המשנה או רק את תיקיית המקור הראשית">?</button>
            </div>
          </fieldset>
        </div>

        <div class="option-group" id="destination-options-group">
          <span class="group-title">מבנה היעד</span>
          <fieldset>
            <legend class="visually-hidden">מתקדם</legend>
            <div class="option-row">
              <label class="option">
                <input id="singles_folder" type="checkbox">
                <span id="singles_folder-label">צור תיקיות סינגלים</span>
              </label>
              <button class="help-badge" aria-label="עזרה: צור תיקיות סינגלים" data-help-text="יצירת תיקייה ייעודית בשם ״סינגלים״ בתוך כל תיקיית אמן">?</button>
            </div>
            <div class="option-row">
              <label class="option">
                <input id="abc_sort" type="checkbox">
                <span id="abc_sort-label">מיון אלפביתי</span>
              </label>
              <button class="help-badge" aria-label="עזרה: מיון אלפביתי" data-help-text="יצירת תיקיות ראשיות לפי אותיות הא׳-ב׳ וארגון תיקיות האמנים בתוכן">?</button>
            </div>
            <div class="option-row">
              <label class="option">
                <input id="exist_only" type="checkbox">
                <span id="exist_only-label">שימוש בתיקיות קיימות</span>
              </label>
              <button class="help-badge" aria-label="עזרה: שימוש בתיקיות קיימות" data-help-text="העברת קבצים רק לתיקיות אמנים הקיימות כבר בתיקית היעד">?</button>
            </div>
          </fieldset>
        </div>
      </div>

      <div class="options-bottom">
        <button id="save-settings" class="text-button">שמור כברירת מחדל</button>
      </div>
      <p id="settings-notice" role="status" class="hint"></p>
    </aside>
  </div>

  <section id="processing-screen" hidden aria-labelledby="processing-title">
    <span class="equalizer" aria-hidden="true"><i></i><i></i><i></i><i></i></span>
    <progress aria-label="מתבצעת בדיקה"></progress>
    <h2 id="processing-title" tabindex="-1">קורא ומנתח את הקבצים…</h2>
    <p id="busy" role="status" aria-live="polite"></p>
    <button id="cancel-preview" class="text-button">ביטול</button>
  </section>

  <section id="results" class="results card" aria-labelledby="results-heading">
    <div class="results-toolbar">
      <div class="toolbar-nav">
        <button id="back-setup" class="text-button">חזרה</button>
        <div class="section-title">
          <h2 id="results-heading" tabindex="-1">תוכנית מיון</h2>
          <span id="count" class="quiet-label" role="status"></span>
        </div>
      </div>
    </div>

    <section id="execution-panel" class="execution-panel" aria-label="ביצוע התוכנית" hidden>
      <div class="execution-heading">
        <div>
          <h3 id="execution-title">מוכן לביצוע</h3>
          <p id="execution-summary" role="status" aria-live="polite"></p>
          <div id="execution-stats" class="execution-stats" aria-hidden="true" hidden></div>
        </div>
        <button id="review-apply" class="primary">בצע את התוכנית</button>
        <button id="select-album-source-parent" class="secondary" aria-describedby="album-source-parent-help" hidden>בחירת תיקיית האב ובדיקה מחדש</button>
        <button id="cancel-apply" class="secondary" hidden>עצור ביצוע</button>
        <button id="finish-results" class="primary" hidden>סיום</button>
        <button id="show-completed-list" class="text-button" hidden>הצג את הקבצים</button>
      </div>
      <p id="album-source-parent-help" class="hint" hidden>תיקיית האב משמשת רק להסרת תיקיית המקור אם תתרוקן לאחר ההעברה המתוכננת. ודא ששום יישום או משתמש אחר אינו משנה את התיקיות בזמן הביצוע.</p>
      <details id="execution-refusal" class="execution-refusal" hidden>
        <summary>פרטי החסימה</summary>
        <p class="hint">מידע טכני לבירור הסיבה. ייתכן שמופיעים בו שמות קבצים או מיקומים.</p>
        <pre id="execution-refusal-details" dir="auto"></pre>
      </details>
      <progress id="execution-progress" aria-label="התקדמות הקובץ הנוכחי" hidden></progress>
      <p id="execution-recovery" class="warning" hidden></p>
      <p id="recovery-inspection" class="hint" hidden></p>
      <label id="journal-label" class="detail-path-label" hidden>
        יומן הפעולה
        <input id="journal-path" class="source-path" dir="ltr" readonly>
      </label>
    </section>

    <div class="results-filterbar">
      <div class="filter-scope"><span id="filter-summary" role="status" aria-live="polite"></span></div>
      <div class="results-filters" role="group" aria-label="סינון בעמוד הנוכחי">
        <button type="button" class="filter-chip active" aria-pressed="true" data-filter="all" id="filter-chip-all">הכל</button>
        <button type="button" class="filter-chip" aria-pressed="false" data-filter="ready" id="filter-chip-ready">עם פעולה</button>
        <button type="button" class="filter-chip" aria-pressed="false" data-filter="attention" id="filter-chip-attention">דורש בדיקה</button>
        <button type="button" class="filter-chip" aria-pressed="false" data-filter="inactive" id="filter-chip-inactive">ללא פעולה</button>
      </div>
      <p class="filter-note">הסינון משנה את התצוגה בלבד, הביצוע חל על התוכנית המלאה</p>

      <label class="option inactive-option">
        <input id="show-inactive" type="checkbox">
        <span>הצג גם ללא פעולה</span>
        <span id="inactive-count" class="quiet-label"></span>
      </label>
    </div>

    <div class="results-body" id="results-body" tabindex="0" aria-label="תוכן התוצאות">
      <div id="empty" class="empty">התוצאה תופיע כאן לאחר בחירת תיקיות והפעלת בדיקה.</div>
      <div id="warnings"></div>
      <p id="filter-empty" class="empty" hidden>אין פריטים שמתאימים לסינון בעמוד הזה</p>
      <div id="table-wrap" hidden>
        <table>
          <thead>
            <tr>
              <th scope="col">שם הקובץ</th>
              <th scope="col" id="result-second-heading">אמן</th>
              <th scope="col" id="result-third-heading">מצב</th>
              <th scope="col"><span class="visually-hidden">פרטים</span></th>
            </tr>
          </thead>
          <tbody id="rows"></tbody>
        </table>
      </div>

      <details id="details" hidden>
        <summary>פרטי התכנית והבדיקה</summary>
        <dl>
          <dt>מזהה תכנית</dt>
          <dd id="plan-id" dir="ltr"></dd>
          <dt>חסימות ביצוע</dt>
          <dd id="blockers"></dd>
        </dl>
        <div id="technical-notices"></div>
      </details>
    </div>

    <nav id="pagination" aria-label="עמודי תוצאות" hidden>
      <button id="previous" class="secondary">הקודם</button>
      <span id="page-label"></span>
      <button id="next" class="secondary">הבא</button>
    </nav>
  </section>
</main>

<footer>
  להורדת סינגלים חינם · <button id="footer-link" class="footer-link">שיר בוט</button>
</footer>

<dialog id="recovery-dialog" class="wide-dialog recovery-dialog" aria-labelledby="recovery-title">
  <div class="dialog-heading">
    <h2 id="recovery-title">בדיקת פעולות קודמות</h2>
    <button id="recovery-close" class="icon-button" aria-label="סגור יומנים">×</button>
  </div>
  <p class="hint">בחר פעולה ואת התיקיות שלה כדי לבדוק את מצב הקבצים. הבדיקה אינה משנה דבר.</p>
  <p id="recovery-error" class="error" role="alert" hidden></p>
  <p id="recovery-busy" role="status" aria-live="polite" hidden>בודק את היומן והקבצים…</p>
  <div class="dialog-actions">
    <button id="recovery-cancel" class="secondary" hidden>עצור בדיקה</button>
    <button id="recovery-refresh" class="secondary">רענן יומנים</button>
  </div>
  <div id="recovery-list" class="recovery-list" aria-label="יומני פעולות"></div>
  <nav id="recovery-pagination" class="list-pagination" aria-label="עמודי יומנים" hidden>
    <button id="recovery-prev" class="secondary">הקודם</button>
    <span id="recovery-range"></span>
    <button id="recovery-next" class="secondary">הבא</button>
  </nav>
  <p id="recovery-truncated" class="warning" hidden>הרשימה מוגבלת למספר קבוע של יומנים. יומנים נוספים נשמרו בתיקיית נתוני התוכנה.</p>
  <section id="recovery-selection" hidden>
    <p id="recovery-record-detail" class="hint"></p>
    <h3>התיקיות שתועדו ביומן</h3>
    <label class="detail-path-label">מקור<input id="recovery-recorded-source" class="source-path" dir="ltr" readonly></label>
    <label class="detail-path-label">יעד<input id="recovery-recorded-target" class="source-path" dir="ltr" readonly></label>
    <div id="recovery-source-mode" hidden>
      <label for="recovery-source-location">מיקום המקור כעת</label>
      <select id="recovery-source-location">
        <option value="present">תיקיית המקור קיימת</option>
        <option value="removed">תיקיית המקור כבר לא קיימת</option>
      </select>
      <label class="detail-path-label">התיקייה שהכילה את המקור<input id="recovery-recorded-parent" class="source-path" dir="ltr" readonly></label>
      <p class="hint">אם המקור הועבר בשלמותו, בחר את התיקייה שהכילה אותו. המיקום ייבדק בלי לשנות קבצים.</p>
    </div>
    <div class="recovery-roots">
      <button id="recovery-source-picker" class="secondary">בחר תיקית מקור</button>
      <bdi id="recovery-selected-source"></bdi>
      <button id="recovery-target-picker" class="secondary">בחר תיקית יעד</button>
      <bdi id="recovery-selected-target"></bdi>
    </div>
    <button id="recovery-inspect" class="primary">בדוק את הקבצים מול היומן</button>
    <button id="recovery-resume" class="secondary" hidden>המשך בדיקה עם אימות מחדש</button>
  </section>
  <section id="recovery-report" hidden>
    <h3 id="recovery-result-title" tabindex="-1"></h3>
    <p id="recovery-result-summary" role="status"></p>
    <p id="recovery-next-step" class="hint"></p>
    <details id="recovery-evidence" hidden>
      <summary>בדיקות התיקיות ותיעוד הפעולה</summary>
      <div id="recovery-evidence-list"></div>
      <nav id="recovery-evidence-pagination" class="list-pagination" aria-label="עמודי בדיקת תיקיות" hidden>
        <button id="recovery-evidence-prev" class="secondary">הקודם</button>
        <span id="recovery-evidence-range"></span>
        <button id="recovery-evidence-next" class="secondary">הבא</button>
      </nav>
    </details>
    <div id="recovery-operation-list"></div>
    <nav id="recovery-operation-pagination" class="list-pagination" aria-label="עמודי בדיקת קבצים" hidden>
      <button id="recovery-operation-prev" class="secondary">הקודם</button>
      <span id="recovery-operation-range"></span>
      <button id="recovery-operation-next" class="secondary">הבא</button>
    </nav>
  </section>
</dialog>

<dialog id="apply-dialog" aria-labelledby="apply-title">
  <div class="dialog-heading">
    <h2 id="apply-title">לבצע את התוכנית?</h2>
    <button id="apply-dismiss" class="icon-button" aria-label="חזור לתוכנית">×</button>
  </div>
  <p id="apply-description"></p>
  <label class="detail-path-label apply-path">מ<input id="apply-source" class="source-path" dir="ltr" readonly></label>
  <label id="apply-target-label" class="detail-path-label apply-path">אל<input id="apply-target" class="source-path" dir="ltr" readonly></label>
  <p class="hint">הקבצים ייבדקו שוב רגע לפני הביצוע.</p>
  <div class="dialog-actions">
    <button id="confirm-apply" class="primary">אשר ביצוע</button>
    <button id="apply-back" class="secondary">ביטול</button>
  </div>
</dialog>

<dialog id="row-dialog" class="wide-dialog" aria-labelledby="row-title">
  <div class="dialog-heading">
    <h2 id="row-title"></h2>
    <button id="row-close" class="icon-button" aria-label="סגור פרטים">×</button>
  </div>
  <div id="row-content"></div>
</dialog>

<dialog id="menu-dialog" aria-labelledby="menu-title">
  <div class="dialog-heading">
    <h2 id="menu-title">עזרה ומידע</h2>
    <button class="close-dialog icon-button" aria-label="סגור">×</button>
  </div>
  <div class="menu-items" role="group" aria-label="נושאי מידע">
    <button data-help="help" aria-pressed="true" aria-controls="usage-content">איך משתמשים?</button>
    <button data-help="about" aria-pressed="false" aria-controls="about-content">אודות</button>
  </div>
  <button id="recovery-menu" class="secondary" hidden>פעולות קודמות</button>
  <div id="help-content" class="help-content">
    <section id="usage-content" data-info-page="help" aria-labelledby="usage-title">
      <h3 id="usage-title">שלושה צעדים</h3>
      <ol class="usage-steps">
        <li>בחר תיקית מקור, ובמיון גם תיקית יעד.</li>
        <li>לחץ על <strong>מיין שירים</strong> או <strong>תקן שמות</strong>.</li>
        <li>עיין בתוכנית ואשר את הביצוע. שום קובץ לא משתנה לפני האישור.</li>
      </ol>
    </section>
    <section id="about-content" data-info-page="about" aria-labelledby="about-name" hidden>
      <div class="about-brand">
        <img src="/icon.png" width="44" height="44" alt="">
        <div>
          <h3 id="about-name">מסדר הסינגלים</h3>
        </div>
      </div>
      <dl class="about-facts">
        <div><dt>גרסת התוכנה</dt><dd id="about-version" dir="ltr"></dd></div>
        <div><dt>פרויקט המקור</dt><dd><bdi>NHLOCAL / Singles-Sorter</bdi></dd></div>
      </dl>
      <div class="about-links">
        <button class="secondary" data-project-link="Source">קוד המקור ב־GitHub</button>
        <button class="secondary" data-project-link="License">רישיון MIT</button>
      </div>
      <p id="about-link-error" class="error" role="alert" hidden></p>
      <button class="secondary" data-local-notices hidden>הודעות רישוי וקוד מקור לרכיבים</button>
      <details class="about-credits">
        <summary>רכיבים ותודות</summary>
        <p>מבוסס על מסדר הסינגלים המקורי של NHLOCAL. נבנה עם <bdi>Rust</bdi>, <bdi>Tauri</bdi>, <bdi>TagLib</bdi> ו־<bdi>Fluent UI Icons</bdi>. לרכיבי צד שלישי תנאי רישוי משלהם.</p>
      </details>
    </section>
  </div>
</dialog>

<dialog id="advanced-dialog" aria-labelledby="advanced-title">
  <div class="dialog-heading">
    <h2 id="advanced-title">הגדרות</h2>
    <button class="close-dialog icon-button" aria-label="סגור">×</button>
  </div>
  <fieldset>
    <legend>מיון דואטים <button class="help-badge" aria-label="עזרה: דואטים" data-help-text="בחר להיכן יועתקו שירי דואט. הבחירה נשמרת מיד, בלי לשמור שינויים באפשרויות האחרות.">?</button></legend>
    <label class="option"><input type="radio" name="duet" id="duet-first" value="first" checked><span>לזמר הראשון בשם השיר</span></label>
    <label class="option"><input type="radio" name="duet" id="duet-all" value="all"><span>לכל הזמרים המופיעים בשיר</span></label>
  </fieldset>
  <fieldset>
    <legend class="ai-heading">
      <span>זיהוי אמנים בעזרת AI</span>
      <span id="model-status" class="quiet-label"></span>
      <button id="model-help" class="help-badge" aria-label="עזרה: מצב מודל ה-AI" data-help-text="">?</button>
    </legend>
    <div class="field-label visually-hidden">
      <label for="ai-profile">שימוש ב-AI</label>
      <button class="help-badge" aria-label="עזרה: שימוש ב-AI" data-help-text="כשהאפשרות פעילה, מודל AI מזהה אמנים שאינם מופיעים ברשימה. אם המודל אינו מותקן, הבדיקה לא תתחיל.">?</button>
    </div>
    <select id="ai-profile">
      <option value="Disabled">כבוי – זיהוי לפי הרשימה בלבד</option>
      <option value="Configured">פעיל – דורש מודל מותקן</option>
    </select>
  </fieldset>
  <section>
    <div class="field-label">
      <h3>רשימת זמרים אישית</h3>
      <button class="help-badge" aria-label="עזרה: רשימת אמנים" data-help-text="השורות האישיות מתווספות אחרי הרשימה המובנית. סדר ההתאמה נשמר.">?</button>
    </div>
    <div class="list-actions">
      <button id="edit-list" class="secondary">ערוך רשימה</button>
      <button id="import-list" class="secondary">יבא קובץ CSV</button>
      <button id="export-list" class="secondary">יצא לקובץ CSV</button>
    </div>
    <p id="profile-notice" class="hint" role="status"></p>
    <p id="profile-error" class="error" role="alert" hidden></p>
  </section>
  <fieldset class="display-settings">
    <legend>מתקדם</legend>
    <label class="option">
      <input id="developer-mode" type="checkbox" role="switch">
      <span>מצב מפתחים</span>
    </label>
    <button id="execution-diagnostics" class="secondary" hidden>שמור יומן פעולה לבדיקה</button>
  </fieldset>
  <p id="settings-error" class="error" role="alert" hidden></p>
  <button class="close-dialog primary">סגור</button>
</dialog>

<dialog id="list-dialog" class="wide-dialog editor-dialog" aria-labelledby="list-title">
  <div class="dialog-heading">
    <h2 id="list-title">הוספת זמרים</h2>
    <button id="list-cancel-x" class="icon-button" aria-label="סגור בלי לשמור">×</button>
    <button class="help-badge" aria-label="עזרה: עריכת רשימה" data-help-text="שם האמן הוא הטקסט לחיפוש; שם התיקייה הוא היעד. אין שינוי סדר או הסרת כפילויות אוטומטיים.">?</button>
  </div>
  <div class="editor-content">
    <div class="editor-config">
      <p id="list-error" class="error" role="alert" hidden></p>
      <p id="list-notice" role="status"></p>
    </div>
    <div class="editor-row-scroll" tabindex="0" aria-label="שורות לעריכה">
      <div class="editor-columns" aria-hidden="true">
        <span></span>
        <span>שם תיקיה</span>
        <span>שם זמר</span>
        <span></span>
      </div>
      <div id="list-rows"></div>
    </div>
  </div>
  <div class="editor-footer">
    <nav class="list-pagination" aria-label="עמודי רשימת אמנים">
      <button id="list-prev" class="secondary">הקודם</button>
      <span id="list-range"></span>
      <button id="list-next" class="secondary">הבא</button>
    </nav>
    <div class="dialog-actions">
      <button id="list-add" class="secondary">הוסף שורה</button>
      <button id="list-save" class="primary">שמור רשימה</button>
      <button id="list-cancel" class="secondary">ביטול</button>
    </div>
  </div>
</dialog>

<dialog id="import-dialog" class="wide-dialog editor-dialog" aria-labelledby="import-title">
  <div class="dialog-heading">
    <h2 id="import-title">בדיקת קובץ הייבוא</h2>
    <button id="import-cancel-x" class="icon-button" aria-label="בטל ייבוא">×</button>
    <button class="help-badge" aria-label="עזרה: ייבוא רשימה" data-help-text="האישור מוסיף את כל השורות בסוף הרשימה. שורות לא תקינות וכפילויות אינן מוסרות אוטומטית. אפשר לערוך או למחוק שורות לפני האישור.">?</button>
  </div>
  <div class="editor-content">
    <div class="editor-config">
      <p id="import-filename" dir="auto"></p>
      <label for="import-encoding">קידוד הקובץ</label>
      <select id="import-encoding">
        <option value="Utf8">UTF-8</option>
        <option value="Utf8Bom">UTF-8 עם BOM</option>
        <option value="Windows1255">Windows-1255 (עברית)</option>
        <option value="Windows1252">Windows-1252</option>
        <option value="Utf16Le">UTF-16 LE</option>
        <option value="Utf16Be">UTF-16 BE</option>
      </select>
      <button id="decode-import" class="secondary">הצג לפי הקידוד שבחרתי</button>
      <p id="import-error" class="error" role="alert" hidden></p>
      <p id="import-summary"></p>
    </div>
    <div class="editor-row-scroll" tabindex="0" aria-label="שורות לעריכה">
      <div class="editor-columns" aria-hidden="true">
        <span></span>
        <span>שם תיקיה</span>
        <span>שם זמר</span>
        <span></span>
      </div>
      <div id="import-rows"></div>
    </div>
  </div>
  <div class="editor-footer">
    <nav class="list-pagination" aria-label="עמודי הייבוא">
      <button id="import-prev" class="secondary">הקודם</button>
      <span id="import-range"></span>
      <button id="import-next" class="secondary">הבא</button>
    </nav>
    <div class="dialog-actions">
      <button id="confirm-import" class="primary">אשר הוספה לרשימה</button>
      <button id="cancel-import" class="secondary">ביטול</button>
    </div>
  </div>
</dialog>
`;
