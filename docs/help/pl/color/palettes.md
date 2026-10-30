# Palety kolorów

Paleta kolorów wzdłuż dolnej krawędzi okna to najszybszy sposób kolorowania
obiektów. Pod nią **paleta dokumentu** gromadzi kolory używane w rysunku. Możesz
otwierać kolejne palety, tworzyć własne i je edytować.

## Korzystanie z palety {#using}

| Czynność | Wynik |
|---|---|
| Kliknięcie koloru | wypełnia zaznaczenie (gdy nic nie jest zaznaczone, ustawia wypełnienie nowych obiektów) |
| Kliknięcie koloru prawym przyciskiem | koloruje kontur zaznaczenia |
| <kbd>Ctrl</kbd>+kliknięcie koloru | domieszkowuje jedną dziesiątą koloru do wypełnienia zaznaczenia |
| Kliknięcie i przytrzymanie koloru | pokazuje do wyboru jaśniejsze, ciemniejsze, bardziej matowe i jaskrawsze odcienie |
| Przeciągnięcie koloru na obiekt | wypełnia obiekt lub koloruje jego kontur, gdy upuścisz kolor na krawędzi |
| Kliknięcie przekreślonego pola | usuwa wypełnienie (prawy przycisk: kontur) |

Strzałki na końcach przewijają paletę, a przycisk na prawym końcu pokazuje
wszystkie kolory naraz. Zatrzymaj wskaźnik nad kolorem, aby zobaczyć jego nazwę
i wartości. Przycisk menu palety na lewym końcu otwiera menu palety.

Paletę można przenieść na inną krawędź okna lub uczynić pływającą: przeciągnij
ją za kropkowany uchwyt.

Z każdej strony może leżeć przy krawędzi okna albo obok rysunku, między rysunkiem a panelami lub przybornikiem po tej stronie. Podczas przeciągania niebieski pas pokazuje, gdzie trafi; kliknij uchwyt prawym przyciskiem, aby wybrać miejsce z listy.

## Paleta dokumentu {#document-palette}

Każdy kolor zastosowany do obiektu trafia do palety dokumentu, która jest
zapisywana z rysunkiem. Jej kroplomierz dodaje kolor spod wskaźnika, a jej menu
zawiera polecenia **Aktualizuj automatycznie** (aby kolory przestały dodawać się
same), **Dodaj z zaznaczenia**, **Dodaj z dokumentu**, **Usuń kolor** i **Resetuj
paletę** (zachowuje tylko kolory nadal używane w rysunku). Możesz też przeciągać
na nią kolory lub obiekty. Aby ją pokazać lub ukryć, zobacz
[Opcje dostosowywania](../customize/customization.md#color-palette).

## Panel Palety {#palettes-docker}

Panel [[docker.palettes]] zawiera listę posiadanych palet z liczbą kolorów
w każdej z nich. Otwórz go poleceniem
[[menu.window > menu.window.color_palettes > menu.window.palette_manager]].

![Panel Palety](../../img/color-palettes.png)

- Zaznacz paletę, aby wyświetlić ją pod oknem rysunku.
- [[docker.edit]] otwiera ją w edytorze palet.
- [[menu.window.palette_open]] otwiera plik palety (`.tdpal`, `.gpl`, `.ase`
  lub `.aco`).
- [[menu.window.palette_from_document]] i
  [[menu.window.palette_from_selection]] tworzą paletę z kolorów użytych
  w rysunku lub w zaznaczeniu.
- Kolory palety dokumentu są widoczne u dołu; kliknij jeden z nich, aby wypełnić
  zaznaczenie, lub kliknij prawym przyciskiem, aby pokolorować kontur.

## Edytowanie palety {#palette-editor}

[[menu.window > menu.window.color_palettes > menu.window.palette_editor]]
otwiera edytor palet.

![Edytor palet](../../img/color-palettes-palette-editor.png)

1. Wybierz paletę w polu [[dialog.palette]] do edycji lub kliknij
   [[dialog.new_palette]], aby utworzyć nową.
2. Wpisz jej nazwę w polu [[dialog.name]].
3. Kliknij kolor, aby go zaznaczyć; [[dialog.add_color]] dodaje bieżący kolor,
   a [[dialog.remove_color]] usuwa zaznaczony.
4. Kliknij [[dialog.save_palette]], aby zapisać ją jako plik.
5. Kliknij [[dialog.close]].

Palet wbudowanych nie można edytować; utwórz nową paletę lub edytuj kopię.
