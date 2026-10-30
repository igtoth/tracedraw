# Przezroczystość

Przezroczystość pozwala, aby to, co leży pod obiektem, prześwitywało przez
niego. Może być jednakowa na całej powierzchni, zmieniać się w obrębie obiektu
jak gradient lub odpowiadać wzorowi albo teksturze.

## Stosowanie przezroczystości {#apply}

- Narzędziem [[tool.transparency]]: kliknij obiekt, a następnie wybierz rodzaj
  przezroczystości na pasku właściwości lub przeciągnij przez obiekt, aby
  uzyskać przezroczystość gradientową od nieprzezroczystej (w miejscu
  naciśnięcia) do przezroczystej (w miejscu zwolnienia). Węzły gradientu pojawiają się wtedy na obiekcie, jak w narzędziu [[tool.interactive_fill]]: przeciągnij początek, koniec lub okrągły węzeł pochylenia; przeciągnij pasek między dwoma węzłami (punkt środkowy), aby przesunąć miejsce, w którym przejście jest w połowie; dwukrotne kliknięcie linii dodaje węzeł, a węzła go usuwa.
- W sekcji [[docker.transparency]] panelu [[docker.properties]].

## Ustawienia {#settings}

- Rodzaj: [[transparency.none]], [[transparency.uniform]] (domyślnie),
  [[transparency.fountain]] (liniowa, promieniowa, stożkowa, kwadratowa lub
  kształtowa, jak wypełnienia gradientowe), [[transparency.pattern]] lub
  [[transparency.texture]].
- Wartość: od 0 do 100% (domyślnie 50%). W gradiencie punkty koloru są
  odcieniami szarości: biel jest nieprzezroczysta, czerń całkowicie
  przezroczysta.
- [[transparency.merge_mode]]: sposób mieszania kolorów z tym, co leży pod
  spodem. Normalny (domyślnie) po prostu prześwituje; pozostałe, takie jak
  Dodaj, Odejmij, Różnica, Mnożenie, Dzielenie, Jeśli jaśniejszy, Jeśli
  ciemniejszy, Barwa, Nasycenie, Kolor i Odwróć, łączą kolory na inne sposoby.
- Czy dotyczy [[transparency.target_fill]], [[transparency.target_outline]]
  czy [[transparency.target_all]].
- **Zamroź**: zachowuje to, co prześwituje, w obecnym stanie, nawet jeśli
  obiekty pod spodem się przesuną.

Aby ją usunąć, kliknij [[toolbar.clear_transparency]] na końcu paska właściwości (albo wybierz [[transparency.none]] jako rodzaj).

Aby skopiować przezroczystość innego obiektu, użyj
[[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]].
Obszary prześwitujące określone przez jasność obiektu opisano w sekcji
[Maski krycia](../objects/clip-frames.md#opacity-mask).
