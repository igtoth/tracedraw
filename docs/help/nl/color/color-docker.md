# Het paneel Kleur

Het paneel [[docker.color]] (<kbd>Shift+F11</kbd>) mengt een kleur en past die
toe op de vulling of omtrek van de selectie. Het gereedschap
[[tool.outline_color]] (<kbd>Shift+F12</kbd>) opent het ook. Open het via
[[menu.window > menu.window.dockers]].

![Het paneel Kleur](../../img/color-color-docker.png)

## Een kleur kiezen {#choose}

1. Selecteer objecten. Het paneel begint met de vulkleur van het eerste object.
2. Meng de nieuwe kleur in een van de weergaven:
   - [[docker.color_viewers]]: klik of sleep in het vierkant om de verzadiging
     (van links naar rechts) en de helderheid (van onder naar boven) in te
     stellen, en in de strook ernaast om de tint in te stellen;
   - [[docker.color_sliders]]: één schuifregelaar per component van het
     [[docker.model]]: RGB, CMYK (in het begin), HSB, HSL, Lab, Grijswaarden
     of YIQ;
   - [[docker.color_palettes]]: de stalen van een palet.

   De drie knoppen bovenaan tonen of verbergen elke weergave; elke combinatie
   kan zichtbaar zijn. U kunt ook een waarde `#rrggbb` typen.
3. Klik onderaan het paneel op [[docker.fill]] of [[docker.outline]] om de
   nieuwe kleur toe te passen.
   Het slot ([[docker.lock_colors]]) koppelt vulling en omtrek, zodat elke knop
   de kleur op beide toepast. De knop "..." ([[docker.more_actions]]) opent de
   overige acties: [[docker.no_fill]], [[docker.no_outline_action]],
   [[docker.swap_colors]], [[docker.add_to_document_palette]],
   [[docker.add_to_spot_colors]] en [[docker.add_to_color_styles]].

De oude en de nieuwe kleur staan naast elkaar. Als de nieuwe kleur niet exact
met CMYK-inkten kan worden afgedrukt, toont het paneel een waarschuwing.

## Een kleur opnemen {#sample}

Klik op [[docker.eyedropper]] en klik ergens in de tekening om de kleur op die
plek op te nemen. Klik daarna op [[docker.use_sampled]] om er de nieuwe kleur
van te maken. Zie ook [Pipetten](eyedroppers.md).

## Steunkleurinkten {#spot-inks}

Onder de knoppen toont [[docker.spot_inks]] de steunkleurinkten van de
tekening; zie [Steunkleuren](spot-colors.md).
