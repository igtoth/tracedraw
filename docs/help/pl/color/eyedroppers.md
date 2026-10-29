# Kopiowanie kolorów i atrybutów kroplomierzami

Narzędzia kroplomierza pobierają kolor lub cały zestaw atrybutów z jednego
obiektu i przekazują go innym. Oba działają w dwóch krokach, widocznych na pasku
właściwości: **pobierz** (weź próbkę) i **zastosuj** (maluj nią). Po pobraniu
próbki narzędzie przełącza się na stosowanie; kliknij przycisk pobierania, aby
pobrać próbkę ponownie. <kbd>Esc</kbd> porzuca próbkę i wraca do narzędzia
[[tool.pick]].

Wskaźnik jest pipetą, gdy narzędzie pobiera próbkę, i wiadrem z farbą, gdy ją
nakłada. Obrazy podają swój kolor także w grupie lub ramce.

## Kroplomierz koloru {#color-eyedropper}

1. Wybierz narzędzie [[tool.color_eyedropper]].
2. Kliknij obiekt, aby pobrać jego kolor: kolor wypełnienia jednolitego,
   pierwszy kolor gradientu lub kolor konturu. Na bitmapie pobiera kolor pikseli
   pod wskaźnikiem, uśredniony w obszarze próbki ustawionym na pasku właściwości
   (1 na 1, 2 na 2 lub 5 na 5 pikseli). <kbd>Shift</kbd>+kliknięcie pobiera
   najpierw kolor konturu.
3. Klikaj obiekty, aby wypełnić je tym kolorem; <kbd>Shift</kbd>+kliknięcie
   koloruje ich kontur (obiekt bez konturu otrzymuje cienki kontur). Kliknięcie
   wewnątrz pustego kształtu zamkniętego wypełnia go.

Pasek właściwości pokazuje pobrany kolor.

Gdy przesuwasz wskaźnik, aby pobrać kolor, małe pole obok pokazuje kolor pod
nim: próbkę, nazwę koloru, jeśli jest kolorem palety lub farbą dodatkową, jego
wartości w trybie kolorów rysunku (CMYK lub RGB) i wartość szesnastkową.
Kroplomierz próbnika kolorów pokazuje to samo pole.

## Kroplomierz atrybutów {#attributes-eyedropper}

Narzędzie [[tool.attributes_eyedropper]] kopiuje naraz kilka właściwości
z jednego obiektu do innych.

1. Wybierz narzędzie [[tool.attributes_eyedropper]].
2. Na pasku właściwości wybierz, co skopiować:
   - **Właściwości**: kontur, wypełnienie i tekst (czcionka, rozmiar i styl; sam
     tekst zostaje zachowany). Domyślnie wszystkie włączone.
   - **Przekształcenia**: rozmiar, obrót i położenie. Domyślnie wyłączone.
   - **Efekty**: przezroczystość, cień i efekty dynamiczne. Domyślnie
     wyłączone.
3. Kliknij obiekt, z którego chcesz kopiować.
4. Kliknij każdy obiekt, do którego chcesz kopiować. <kbd>Shift</kbd>+kliknięcie
   pobiera zamiast tego próbkę z innego obiektu.

Każde kliknięcie to jeden krok cofania.
