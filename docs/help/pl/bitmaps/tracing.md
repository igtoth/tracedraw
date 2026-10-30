# Wektoryzacja bitmap

Wektoryzacja zamienia bitmapę w kształty wektorowe, które można edytować,
przekolorowywać i skalować bez utraty jakości: zeskanowane logo, szkic,
zdjęcie.

## Szybka wektoryzacja {#quick-trace}

Zaznacz bitmapę i wybierz [[menu.bitmaps > menu.bitmaps.quick_trace]].
Używa ona ustawienia wstępnego wybranego na stronie
[[options.bitmap_tracing]] opcji globalnych (początkowo Clipart).

[[menu.bitmaps.repeat_last_trace]] wektoryzuje zaznaczoną bitmapę ponownie tak jak ostatnio, bez okna dialogowego, a [[menu.bitmaps.last_trace_settings]] otwiera okno dialogowe z tamtymi ustawieniami. Menu kontekstowe bitmapy też ma te polecenia, razem z [[menu.bitmaps.centerline_trace]] i [[menu.bitmaps.outline_trace]].

## Okno dialogowe Wektoryzuj bitmapę {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] i
[[menu.bitmaps.centerline_trace]] wymieniają ustawienia wstępne; wybierz
jedno, aby otworzyć okno dialogowe.

- **Wektoryzacja konturowa** wypełnia obszary koloru kształtami:
  [[menu.bitmaps.trace_line_art]], [[menu.bitmaps.trace_logo]],
  [[menu.bitmaps.trace_detailed_logo]], [[menu.bitmaps.trace_clipart]],
  [[menu.bitmaps.trace_low_quality]] i [[menu.bitmaps.trace_high_quality]].
- **Wektoryzacja linii środkowej** podąża za liniami otwartymi krzywymi:
  [[menu.bitmaps.trace_technical]] i [[menu.bitmaps.trace_line_drawing]].

![Okno dialogowe Wektoryzuj bitmapę](../../img/bitmaps-tracing.png)

Okno dialogowe można powiększać i maksymalizować. Podgląd wypełnia jego
lewą stronę, ustawienia leżą w panelu po prawej, a wynik jest tworzony od
nowa przy każdej zmianie ustawienia: nie trzeba niczego naciskać, aby go
zobaczyć. Pod podglądem wiersz liczy **krzywe**, **węzły** i **kolory**
wyniku. Kliknij [[dialog.ok]], aby umieścić kształty nad bitmapą,
najciemniejsze najpierw, lub [[dialog.cancel]], aby zostawić bitmapę bez
zmian.

## Podgląd {#preview}

Lista [[dialog.trace_preview_label]] wybiera, co pokazuje podgląd:

- [[dialog.trace_view_before_after]]: bitmapa u góry i jej wynik poniżej,
  albo [[dialog.view_side_by_side]]; [[dialog.before]] pokazuje samą
  bitmapę.
- [[dialog.trace_view_large]]: sam wynik, wielkości całego podglądu.
- [[dialog.trace_view_wireframe]]: krzywe wyniku narysowane cienkimi
  liniami na bitmapie. Obok listy pojawia się suwak [[dialog.trace_overlay]]:
  ustala, jak mocno bitmapa prześwituje pod liniami, od 0 (same linie) do
  100 (bitmapa bez zmian), początkowo 80.

Przyciski za listą dopasowują obraz do okna, oddalają, przybliżają i
przesuwają. Kółko myszy powiększa wokół wskaźnika, a przeciąganie przesuwa
obraz; oba pola poruszają się razem. Przezroczyste części pokazują
szachownicę.

## Ustawienia {#settings}

Karta [[dialog.trace_tab_settings]] zawiera:

- [[dialog.trace_type]] Kontur lub Linia środkowa, oraz
  [[dialog.trace_image_type]] ustawienie wstępne. Wybór ustawia suwaki
  poniżej na wartości odpowiednie dla tego rodzaju obrazu.
- [[dialog.detail]] (0 do 100): więcej szczegółów zachowuje mniejsze
  obszary i cieńsze linie; mniej szczegółów wygładza plamki i ziarno.
- [[dialog.smoothing]] (0 do 100, początkowo 25): gładsze krzywe z mniejszą
  liczbą węzłów.
- [[dialog.corner_smoothness]] (0 do 100): bardziej zaokrąglone narożniki.
- [[dialog.trace_remove]]: co pominąć w wyniku. **Kolor tła** usuwa tło
  tam, gdzie sięga krawędzi obrazu, **Cały kolor z obrazu** usuwa każdy
  obszar tego koloru, **Brak** wektoryzuje wszystko.
  [[dialog.trace_remove_auto]] znajduje kolor tła wzdłuż krawędzi obrazu;
  [[dialog.trace_remove_specify]] używa koloru próbki, który można zmienić
  lub pobrać z podglądu pipetą obok.
- [[dialog.trace_objects]]: [[dialog.trace_merge_adjacent]] (początkowo
  włączone) robi jedną krzywą z każdego obszaru koloru; wyłączone, każda
  osobna plama jest własną krzywą. [[dialog.trace_remove_overlap]] trzyma
  każdy kształt na jego własnych pikselach; wyłączone, kształty sięgają
  nieco pod te nad nimi, aby między sąsiadami nie było szczelin.
  [[dialog.trace_group_by_color]] umieszcza krzywe każdego koloru w
  osobnej grupie; jest dostępne, gdy obszary koloru nie są już scalone.
- [[dialog.delete_original]]: bitmapa znika, gdy wynik zostaje
  umieszczony.

Ustawienia, które nie dotyczą bieżącego typu wektoryzacji, są wyszarzone:
wektoryzacja linii środkowej czyta tylko szczegóły i wygładzanie.

## Kolory {#colors}

Karta [[dialog.trace_tab_colors]] ustawia [[dialog.color_mode]] (RGB,
skala szarości, lub czarno-biały) i [[dialog.trace_number_of_colors]] (2 do
256) oraz wymienia kolory wyniku z liczbą krzywych każdego z nich. Zaznacz
dwa lub więcej kolorów i kliknij [[dialog.trace_merge_colors]], aby
połączyć je w pierwszy; kliknij próbkę koloru, aby rysować jego krzywe
innym kolorem. Zmiana ustawienia, które wektoryzuje obraz od nowa, zapomina
te zmiany, bo należały do poprzedniego wyniku.

## Resetowanie, cofanie i ponawianie {#steps}

Pod panelem ustawień [[dialog.reset]] przywraca ustawienia ustawienia
wstępnego, a dwie strzałki cofają się i idą naprzód przez każdą zmianę
wykonaną w oknie dialogowym: każde puszczenie suwaka, wybór z listy, pole
wyboru, zmiana koloru i reset to jeden krok. Zmiana wykonana po cofnięciu
odrzuca cofnięte kroki.

[[dialog.trace_options]] otwiera stronę [[options.bitmap_tracing]] opcji
globalnych, gdzie wybiera się ustawienie wstępne szybkiej wektoryzacji i
wartości początkowe każdej wektoryzacji, po czym wraca do okna dialogowego.
Przycisk [[dialog.help_button]] otwiera tę stronę.
