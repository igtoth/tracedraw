# Text documents (TXT, RTF, DOCX)

File > Open and File > Import accept plain text, Rich Text Format and
Office Open XML word-processing files. The whole document becomes one
paragraph text frame: on Open inside an A4 page with 20 mm margins, on
Import inside the current page with 20 mm margins (a quarter of the page
at most). Paragraphs are separated by paragraph breaks; line breaks
(`\line`, `w:br`) become U+2028; tabs are kept.

| Format | What is kept |
|---|---|
| TXT | UTF-8 (with or without BOM), UTF-16 with BOM, otherwise Windows-1252; CR, LF and CRLF paragraphs; Arial 12 pt |
| RTF | runs with bold, italic, underline, size (`\fs`, half points) and the font table's family; `\'hh` code-page bytes and `\uN` escapes with `\ucN` skips; the first paragraph's alignment (`\ql`, `\qc`, `\qr`, `\qj`); typographic quote, dash, bullet and space control words; `\fonttbl`, `\colortbl`, `\stylesheet`, `\info`, `\pict`, `\object` and `{\*\...}` destinations contribute no text |
| DOCX | `w:p` paragraphs and `w:r` runs from `word/document.xml`: `w:b`, `w:i`, `w:u`, `w:strike`, `w:sz` (half points), `w:rFonts` (ascii or hAnsi), `w:t`, `w:tab`, `w:br`, `w:cr`; the first paragraph's `w:jc` alignment |

Styles defined elsewhere (RTF style sheet, DOCX `styles.xml`) are not
resolved: runs without explicit properties use Arial 12 pt. Text is
capped at 4 million characters.
