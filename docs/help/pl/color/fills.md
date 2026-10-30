# Wypełnienia

Wypełnienie maluje wnętrze obiektu zamkniętego. TraceDraw ma sześć rodzajów
wypełnienia: [[docker.fill_none]], [[docker.fill_uniform]] (jeden płaski kolor),
[[docker.fill_fountain]] (kolory płynnie przechodzące jeden w drugi),
[[docker.fill_pattern]], [[docker.fill_texture]] i [[docker.fill_mesh]]
(kolory płynące między węzłami siatki).

## Sposoby wypełniania obiektu {#ways}

- Kliknij kolor palety, aby nadać zaznaczeniu wypełnienie jednolite.
- Użyj sekcji [[docker.fill]] panelu [[docker.properties]] (<kbd>F11</kbd>):
  wybierz rodzaj wypełnienia w rzędzie obrazków, a następnie jego ustawienia.
- Użyj narzędzia [[tool.interactive_fill]] (<kbd>G</kbd>), aby ustawiać gradienty
  bezpośrednio na obiekcie.
- Użyj narzędzia [[tool.area_fill]], aby wypełnić obszar otoczony przez kilka
  obiektów.
- Użyj narzędzia [[tool.mesh_fill]] (<kbd>M</kbd>), aby uzyskać wypełnienie
  siatkowe.

Gdy nic nie jest zaznaczone, wybrane wypełnienie staje się wypełnieniem nowych
obiektów.

## Wypełnienia jednolite {#uniform}

Wypełnienie jednolite to jeden kolor w dowolnym modelu kolorów. Wybierz go
z palety, z panelu [[docker.color]] (zobacz [Panel Kolor](color-docker.md)) lub
z sekcji wypełnienia panelu [[docker.properties]].

## Wypełnienia gradientowe {#gradient}

Gradient płynnie łączy dwa lub więcej kolorów. Jego ustawienia w sekcji
wypełnienia panelu [[docker.properties]]:

- Typ: [[fill.linear]] (wzdłuż linii), [[fill.radial]] (od środka na zewnątrz),
  [[fill.conical]] (wokół środka), [[fill.square]] (w rosnących kwadratach) lub
  [[fill.shaped]] (zgodnie z obrysem obiektu, od krawędzi do wewnątrz).
- [[docker.gradient_presets]]: gotowe gradienty, od których można zacząć.
- **Punkty koloru**: kolory i ich położenie wzdłuż gradientu, od 2 do 64.
  Zaznacz punkt koloru, aby zmienić jego kolor lub położenie;
  [[docker.add_stop]] i [[docker.remove_stop]] dodają i usuwają punkty koloru.
  [[docker.import_gradient]] pobiera kolory z pliku gradientu `.ggr`.
- **Kąt** gradientów liniowych, stożkowych i kwadratowych, wartość
  [[docker.edge_pad]] (jaka część każdego końca jest jednolitym kolorem) oraz
  przesunięcie środka gradientów promieniowych, stożkowych i kwadratowych.
- [[docker.repeat]]: [[docker.repeat_none]], [[docker.repeat_sawtooth]] (kolory
  biegną ponownie od początku) lub [[docker.repeat_mirror]] (biegną tam
  i z powrotem), z liczbą [[docker.repeat_count]].
- [[docker.bilinear]]: kolory biegną od pierwszego do ostatniego i z powrotem.
- [[docker.invert_colours]]: kolory biegną w odwrotnej kolejności.

Pasek pod typem pokazuje cały przebieg kolorów tak, jak zostanie narysowany.

## Okno dialogowe Edytuj wypełnienie {#edit-fill}

<kbd>F11</kbd> otwiera okno [[edit_fill.title]] z wypełnieniem zaznaczenia.
Dla gradientu pokazuje podgląd, [[edit_fill.fill_picker]], typ, powtarzanie i
pasek kolorów: kliknij węzeł, aby go wybrać, przeciągnij, aby go przesunąć,
kliknij dwukrotnie, aby dodać lub usunąć węzeł. Pod paskiem: kolor,
przezroczystość, położenie i [[edit_fill.midpoint]] węzła oraz
[[edit_fill.reverse]]. [[edit_fill.transform]] zawiera obrót, środek i
margines krawędzi. **OK** stosuje to, co zmieniono, z [[edit_fill.overprint]]. Gdy okno dialogowe jest otwarte, zaznaczenie pokazuje wypełnienie na bieżąco podczas zmian; **Anuluj** lub <kbd>Esc</kbd> przywraca poprzednie wypełnienie.

![Okno dialogowe Edytuj wypełnienie](../../img/color-fills-edit-fill.png)

## Narzędzie Wypełnienie interaktywne {#interactive-fill}

Narzędzie [[tool.interactive_fill]] (<kbd>G</kbd>) edytuje wypełnienia
bezpośrednio na obiekcie.

1. Wybierz narzędzie i przeciągnij przez obiekt: otrzymasz gradient liniowy od
   miejsca naciśnięcia do miejsca zwolnienia przycisku. Przytrzymaj
   <kbd>Ctrl</kbd>, aby ograniczyć kąt do kroków co 15 stopni. Kliknięcie daje
   wypełnienie domyślne.
2. Przeciągaj węzły na obiekcie, aby zmienić gradient: dwa kwadratowe węzły
   gradientu liniowego zmieniają jego kierunek i długość; węzeł gradientu
   promieniowego, stożkowego lub kwadratowego przesuwa jego środek.
3. Przeciągnij kolor palety na węzeł, aby zmienić jego kolor, lub na linię
   między węzłami, aby dodać tam kolor.

Pasek właściwości zawiera rodzaj wypełnienia i jego ustawienia: kolor
wypełnienia jednolitego; dla gradientu typ, kolor początkowy i końcowy, kąt
oraz margines krawędzi.

## Narzędzie Wypełnienie obszaru {#area-fill}

Narzędzie [[tool.area_fill]] wypełnia dowolny obszar zamknięty, nawet utworzony
przez kilka nakładających się obiektów, tworząc nowy obiekt w kształcie tego
obszaru. Otaczające go obiekty pozostają bez zmian.

1. Wybierz narzędzie [[tool.area_fill]].
2. Wybierz wypełnienie i kontur na pasku właściwości: **Określ** (kolor, a
   dla konturu również szerokość, obok), **Użyj domyślnego** (wypełnienie i
   kontur nowych obiektów; kolor, gdy jest to brak wypełnienia, aby obszar był
   widoczny) lub **Brak**.
3. Kliknij wewnątrz obszaru.

Nowy obiekt trafia na wierzch aktywnej warstwy i zostaje zaznaczony. Kliknięcie
konturu lub obszaru, którego nic nie otacza, niczego nie tworzy.

## Wypełnienia siatkowe {#mesh-fill}

Wypełnienie siatkowe to siatka węzłów na obiekcie, z których każdy ma własny
kolor i przezroczystość; kolory płyną płynnie między węzłami. Nadaje się do
realistycznego cieniowania.

1. Wybierz narzędzie [[tool.mesh_fill]] (<kbd>M</kbd>) i ustaw na pasku
   właściwości liczbę wierszy i kolumn (domyślnie 2 na 2, maksymalnie 50).
2. Kliknij obiekt. Otrzymuje on siatkę w kolorze swojego wypełnienia.
3. Kliknij węzeł, aby go zaznaczyć, a następnie kliknij kolor palety, aby go
   pokolorować. Przeciągaj węzły, aby wyginać przepływ kolorów.
4. Kliknij dwukrotnie wewnątrz siatki, aby dodać wiersz lub kolumnę przez
   najbliższy węzeł; zaznacz węzeł i naciśnij <kbd>Delete</kbd>, aby usunąć jego
   wiersz i kolumnę.

Pasek właściwości zawiera też opcję **Wygładź kolor**, która łagodniej łączy
kolory, **przezroczystość** zaznaczonego węzła oraz **Wyczyść siatkę**, która
usuwa wypełnienie siatkowe. Zmiana liczby wierszy i kolumn siatki zachowuje jej
kolory.

## Wypełnienia wzorem i teksturą {#pattern-texture}

- Wypełnienie [[docker.fill_pattern]] powtarza kafelek: dwa kolory, wzór
  wektorowy lub bitmapę. Zobacz [Wypełnienia wzorem](patterns.md).
- Wypełnienie [[docker.fill_texture]] to generowana tekstura o naturalnym
  wyglądzie: [[texture.clouds]], [[texture.marble]], [[texture.noise]] lub
  [[texture.wood]], z własnymi kolorami i skalą.

## Kilka wypełnień jednego obiektu {#several-fills}

W sekcji wypełnienia panelu [[docker.properties]] przycisk [[docker.add_fill]]
maluje kolejne wypełnienie na własnym wypełnieniu obiektu, zaczynając od kopii
do zmiany. Kolejne wypełnienia są widoczne jako „Wypełnienie 2”, „Wypełnienie 3”
i tak dalej, każde z własnym edytorem i przyciskiem do usunięcia. W sekcji
konturu można w ten sam sposób dodać kolejne kontury.

Wypełnienia można też malować na konturach; zobacz
[Ustawianie konturów](../lines-shapes/outlines.md#outline-pen).
