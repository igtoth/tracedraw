# Panel Kolor

Panel [[docker.color]] (<kbd>Shift+F11</kbd>) miesza kolor i stosuje go do
wypełnienia lub konturu zaznaczenia. Otwiera go też narzędzie
[[tool.outline_color]] (<kbd>Shift+F12</kbd>). Otwórz go z menu
[[menu.window > menu.window.dockers]].

![Panel Kolor](../../img/color-color-docker.png)

## Wybieranie koloru {#choose}

1. Zaznacz obiekty. Panel zaczyna od koloru wypełnienia pierwszego z nich.
2. Wymieszaj nowy kolor w jednym z widoków:
   - [[docker.color_viewers]]: kliknij lub przeciągnij w kwadracie, aby ustawić
     nasycenie (od lewej do prawej) i jasność (od dołu do góry), a na pasku obok,
     aby ustawić barwę;
   - [[docker.color_sliders]]: jeden suwak na każdą składową modelu
     [[docker.model]]: RGB, CMYK (domyślnie), HSB, HSL, Lab, Skala szarości lub
     YIQ;
   - [[docker.color_palettes]]: próbki palety.

   Trzy przyciski u góry pokazują lub ukrywają każdy widok; widoczna może być
   dowolna ich kombinacja. Możesz też wpisać wartość `#rrggbb`.
3. Kliknij [[docker.fill]] lub [[docker.outline]] u dołu panelu, aby zastosować
   nowy kolor.
   Zamek ([[docker.lock_colors]]) wiąże wypełnienie i kontur, więc każdy z
   przycisków nadaje kolor obu. Przycisk "..." ([[docker.more_actions]]) rozwija
   pozostałe działania: [[docker.no_fill]], [[docker.no_outline_action]],
   [[docker.swap_colors]], [[docker.add_to_document_palette]],
   [[docker.add_to_spot_colors]] oraz [[docker.add_to_color_styles]].

Stary i nowy kolor są widoczne obok siebie. Gdy nowego koloru nie można
dokładnie wydrukować farbami CMYK, panel wyświetla ostrzeżenie.

## Pobieranie koloru {#sample}

Kliknij [[docker.eyedropper]] i kliknij dowolne miejsce rysunku, aby pobrać
znajdujący się tam kolor. Następnie kliknij [[docker.use_sampled]], aby stał się
nowym kolorem. Zobacz też [Kroplomierze](eyedroppers.md).

## Kolory dodatkowe {#spot-inks}

Pod przyciskami [[docker.spot_inks]] zawiera listę kolorów dodatkowych rysunku;
zobacz [Kolory dodatkowe](spot-colors.md).
