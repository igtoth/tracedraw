# Vectoriser des images bitmap

La vectorisation transforme un bitmap en formes vectorielles que vous pouvez
modifier, recolorer et redimensionner sans perte de qualité : un logo
numérisé, un croquis, une photo.

## Vectorisation rapide {#quick-trace}

Sélectionnez un bitmap et choisissez
[[menu.bitmaps > menu.bitmaps.quick_trace]]. Elle utilise le préréglage
choisi sur la page [[options.bitmap_tracing]] des options globales (Clipart
au départ).

[[menu.bitmaps.repeat_last_trace]] vectorise à nouveau le bitmap sélectionné comme la dernière vectorisation, sans boîte de dialogue, et [[menu.bitmaps.last_trace_settings]] ouvre la boîte de dialogue avec ses paramètres. Le menu contextuel d'un bitmap contient aussi ces commandes, avec [[menu.bitmaps.centerline_trace]] et [[menu.bitmaps.outline_trace]].

## La boîte de dialogue Vectoriser le bitmap {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] et
[[menu.bitmaps.centerline_trace]] proposent des préréglages ; choisissez-en
un pour ouvrir la boîte de dialogue.

- **Vectorisation par contour** remplit les zones de couleur avec des
  formes : [[menu.bitmaps.trace_line_art]], [[menu.bitmaps.trace_logo]],
  [[menu.bitmaps.trace_detailed_logo]], [[menu.bitmaps.trace_clipart]],
  [[menu.bitmaps.trace_low_quality]] et [[menu.bitmaps.trace_high_quality]].
- **Vectorisation par axe** suit les lignes avec des courbes ouvertes :
  [[menu.bitmaps.trace_technical]] et [[menu.bitmaps.trace_line_drawing]].

![La boîte de dialogue Vectoriser le bitmap](../../img/bitmaps-tracing.png)

La boîte de dialogue est une fenêtre que vous pouvez redimensionner et
agrandir. L'aperçu occupe sa partie gauche, les paramètres se trouvent dans
un panneau à droite, et la vectorisation est refaite à chaque changement de
paramètre : rien à presser pour voir le résultat. Sous l'aperçu, une ligne
compte les **courbes**, les **nœuds** et les **couleurs** du résultat.
Cliquez sur [[dialog.ok]] pour placer les formes sur le bitmap, les plus
sombres d'abord, ou sur [[dialog.cancel]] pour laisser le bitmap tel quel.

## L'aperçu {#preview}

La liste [[dialog.trace_preview_label]] choisit ce que montre l'aperçu :

- [[dialog.trace_view_before_after]] : le bitmap en haut et sa
  vectorisation en bas, ou [[dialog.view_side_by_side]] ;
  [[dialog.before]] montre le bitmap seul.
- [[dialog.trace_view_large]] : la vectorisation seule, aussi grande que
  l'aperçu.
- [[dialog.trace_view_wireframe]] : les courbes du résultat tracées en
  lignes fines sur le bitmap. Un curseur [[dialog.trace_overlay]] apparaît
  à côté de la liste : il règle à quel point le bitmap apparaît sous les
  lignes, de 0 (les lignes seules) à 100 (le bitmap tel quel), 80 au
  départ.

Les boutons après la liste ajustent l'image à la fenêtre, réduisent,
agrandissent et déplacent la vue. La molette zoome autour du pointeur et un
glissement déplace l'image ; les deux volets bougent ensemble. Les parties
transparentes montrent un damier.

## Paramètres {#settings}

L'onglet [[dialog.trace_tab_settings]] contient :

- [[dialog.trace_type]] Contour ou Axe, et [[dialog.trace_image_type]] le
  préréglage. En choisir un règle les curseurs ci-dessous aux valeurs qui
  conviennent à ce type d'image.
- [[dialog.detail]] (0 à 100) : plus de détail garde les zones plus petites
  et les lignes plus fines ; moins de détail lisse les taches et le grain.
- [[dialog.smoothing]] (0 à 100, 25 au départ) : des courbes plus lisses
  avec moins de nœuds.
- [[dialog.corner_smoothness]] (0 à 100) : des angles plus arrondis.
- [[dialog.trace_remove]] : ce qui est laissé hors du résultat. **Couleur
  d'arrière-plan** retire l'arrière-plan là où il touche le bord de
  l'image, **Couleur entière de l'image** retire chaque zone de cette
  couleur, **Aucun** vectorise tout. [[dialog.trace_remove_auto]] trouve la
  couleur d'arrière-plan le long du bord de l'image ;
  [[dialog.trace_remove_specify]] utilise la couleur de l'échantillon, que
  vous pouvez changer ou prélever dans l'aperçu avec la pipette à côté.
- [[dialog.trace_objects]] : [[dialog.trace_merge_adjacent]] (activé au
  départ) fait une seule courbe de chaque zone d'une couleur ; désactivé,
  chaque tache séparée devient une courbe à part.
  [[dialog.trace_remove_overlap]] garde chaque forme sur ses propres
  pixels ; désactivé, les formes passent un peu sous celles au-dessus, pour
  qu'aucun jour n'apparaisse entre voisines. [[dialog.trace_group_by_color]]
  place les courbes de chaque couleur dans un groupe à part ; il est
  disponible quand les zones d'une couleur ne sont pas déjà fusionnées.
- [[dialog.delete_original]] : le bitmap disparaît quand le résultat est
  placé.

Les paramètres qui ne s'appliquent pas au type de vectorisation en cours
sont grisés : une vectorisation par axe ne lit que le détail et le lissage.

## Couleurs {#colors}

L'onglet [[dialog.trace_tab_colors]] règle le [[dialog.color_mode]] (RVB,
niveaux de gris, ou noir et blanc) et le [[dialog.trace_number_of_colors]]
(2 à 256), et liste les couleurs du résultat avec le nombre de courbes de
chacune. Cochez deux couleurs ou plus et cliquez sur
[[dialog.trace_merge_colors]] pour les réunir dans la première ; cliquez
sur l'échantillon d'une couleur pour tracer ses courbes dans une autre
couleur. Changer un paramètre qui vectorise à nouveau l'image oublie ces
modifications, qui appartenaient au résultat précédent.

## Réinitialiser, annuler et rétablir {#steps}

Sous le panneau des paramètres, [[dialog.reset]] remet les paramètres du
préréglage, et les deux flèches reviennent et avancent sur chaque
changement fait dans la boîte de dialogue : chaque relâchement d'un
curseur, choix dans une liste, case à cocher, modification de couleur et
réinitialisation est une étape. Un changement fait après une annulation
abandonne les étapes annulées.

[[dialog.trace_options]] ouvre la page [[options.bitmap_tracing]] des
options globales, où vous choisissez le préréglage de la vectorisation
rapide et les choix de départ de chaque vectorisation, puis revient à la
boîte de dialogue. Le bouton [[dialog.help_button]] ouvre cette page.
