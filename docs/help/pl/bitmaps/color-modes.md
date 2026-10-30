# Zmiana trybu kolorów bitmap

> *Tłumaczenie maszynowe: ta strona może być mniej aktualna niż wersja angielska.*

[[menu.bitmaps > menu.bitmaps.mode]] przekształca zaznaczone bitmapy do innego
trybu kolorów. Każda zaznaczona bitmapa zmienia się w jednym kroku cofania,
a przezroczystość zostaje zachowana. Tryby z ustawieniami otwierają okno
dialogowe z podglądem przed zmianą i po niej.

| Tryb | Wynik |
|---|---|
| [[menu.bitmaps.mode_bw]] | tylko czerń i biel (patrz niżej) |
| [[menu.bitmaps.mode_grayscale]] | odcienie szarości |
| [[menu.bitmaps.mode_duotone]] | od jednej do czterech farb (patrz niżej) |
| [[menu.bitmaps.mode_paletted]] | najwyżej 256 kolorów (patrz niżej) |
| [[menu.bitmaps.mode_rgb]] | tryb bitmapy zmienia się na RGB; piksele zostają bez zmian |
| [[menu.bitmaps.mode_lab]] | kolory Lab |
| [[menu.bitmaps.mode_cmyk]] | tryb bitmapy zmienia się na CMYK, a jej piksele pokazują kolory, które farby mogą wydrukować, obliczone na podstawie profili zarządzania kolorami |

Menu zaznacza [[menu.bitmaps.mode_rgb]] lub [[menu.bitmaps.mode_cmyk]], gdy wszystkie zaznaczone bitmapy są w tym trybie, a pasek stanu podaje tryb zaznaczonej bitmapy, na przykład „Bitmap 600 x 400 px (CMYK)”. Obraz CMYK z pliku `.cdr` otwiera się w trybie CMYK, a pliki `.tdraw` zachowują tryb. [[menu.tools > menu.tools.convert_colors]] ustawia tak samo tryb każdej konwertowanej bitmapy (zobacz [Konwersja kolorów](../color/convert-colors.md)).

## Czerń i biel {#black-and-white}

![Okno dialogowe Czerń i biel](../../img/bitmaps-color-modes-black-and-white.png)

Wybierz metodę konwersji:

- **Grafika kreskowa**: biel tam, gdzie szarość jest równa **progowi** lub
  wyższa (domyślnie 128), czerń w pozostałych miejscach;
- **Uporządkowane**: regularny wzór kropek;
- **Raster półtonowy** (domyślnie): raster okrągłych, liniowych lub kwadratowych
  kropek pod **kątem** (45 stopni) i z liczbą **linii na cal** (60);
- **Cardinality-distribution**: tekstura o przypadkowym wyglądzie;
- **Jarvis**, **Stucki** i **Floyd-Steinberg**: dyfuzja błędu, najgładsze
  wyniki dla zdjęć.

**Intensywność** (od 0 do 100, domyślnie 50) rozjaśnia lub przyciemnia wynik.

## Dwutonowy {#duotone}

![Okno dialogowe Dwutonowy](../../img/bitmaps-color-modes-duotone.png)

Obraz dwutonowy drukuje obraz w skali szarości kilkoma farbami, co daje bogate
tony przy niewielu kolorach.

1. Wybierz typ: Jednotonowy, Dwutonowy (domyślnie), Trójtonowy lub
   Czterotonowy, dla od jednej do czterech farb.
2. Kliknij farbę, aby zmienić jej kolor.
3. Edytuj jej krzywą: w poziomie jest poziom szarości (czerń po lewej), w pionie
   ilość tej farby, która zostanie wydrukowana. Przeciągnij w pustym miejscu,
   aby dodać punkt, przeciągnij punkt, aby go przesunąć, kliknij go dwukrotnie
   lub przeciągnij poza siatkę, aby go usunąć. **Pokaż wszystkie** rysuje też
   krzywe pozostałych farb.
4. **Zapisz** i **Wczytaj** przechowują farby i krzywe w pliku.

## Z paletą {#paletted}

![Okno dialogowe Z paletą](../../img/bitmaps-color-modes-paletted.png)

- **Paleta**: Kolory jednolite, Standardowa VGA (16 kolorów), Adaptacyjna
  i Optymalizowana (własne kolory obrazu, zredukowane do ustawionej liczby
  **kolorów**, maksymalnie 256), Skala szarości, Systemowa lub Własna (paleta
  dokumentu).
- **Roztrząsanie**: Brak, Uporządkowane, Jarvis, Stucki lub Floyd-Steinberg,
  z intensywnością roztrząsania.

Wynikowa paleta jest widoczna pod ustawieniami.

## Kolorowanie bitmap czarno-białych {#monochrome}

Gdy zaznaczone są tylko bitmapy dwukolorowe, kliknij kolor palety, aby
pokolorować ich tło, a kliknij prawym przyciskiem, aby pokolorować ich pierwszy
plan.
