"""Owned, deterministic synthetic boundary suite. No random seed or corpus."""
SMOKE=['', 'ומוטי וייס - שיר חדש', 'אברהם  פריד\n🎵', 'https://example.com/test.mp3', 'İ Σ ς １２٣ ²']
BOUNDARY=SMOKE+[
' ', '\t\r\n', 'א', 'א ב', 'א ב ג', 'א  ב  ג', 'א\nב\tג',
'שיר חדש של זמר לדוגמה', 'אלבום חדש זמר אמן להקה', 'רוק פופ ג׳אז מזרחי',
'Alpha Beta Gamma', 'The Example Band - New Song', 'song album singer genre misc',
'"אברהם" (פריד) - ״שיר״', 'וייס ומוטי ווייס ובית', 'abc@example.com https://example.org/a?x=1',
'👩🏽\u200d🎤 🎵 😀', 'שָׁלוֹם עוֹלָם', '\u200fא\u200eב\u202eג',
'İSTANBUL ΟΣ ΑΣΑ ς σ Σ', '１２٣ ² Ⅷ ½ 𝟙', '€ £ — – …',
'a'*99+' '+'B'*100+' '+'ג'*101,
'אמן ראשון ושיר שני עם אלבום שלישי בשנת 2026',
'א ב ג ד ה ו ז ח ט י כ ל מ נ ס ע פ צ ק ר ש ת',
'א.ב ג-ד ה/ו (ז) [ח] {ט}', 'hello\r\nworld\n\nend',
]
assert len(BOUNDARY)==32
FORCED=['', 'א', 'א ב', 'א ב ג', 'א ב ג ד', 'א  ב', 'א\nב', ' א', 'א ', 'א \n ב']
