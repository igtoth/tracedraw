# Options de TraceDraw

[[menu.tools > menu.tools.options > menu.tools.options_app]]
(<kbd>Ctrl+J</kbd>) règle le fonctionnement du programme. Choisissez une
page dans la liste de gauche.

## Général {#general}

![La page Général des options de TraceDraw](../../img/customize-options-app-general.png)

- [[options.on_startup]] : afficher l'écran d'accueil, commencer un nouveau
  document ou ouvrir le dernier document modifié.
- [[options.show_new_document_dialog]] (actif) : demander les réglages d'un
  nouveau dessin ; voir [Créer un dessin](../get-started/new-document.md).
- [[options.show_splash]] (actif).
- [[options.hardware_acceleration]] (activé) : la fenêtre est dessinée avec la carte graphique choisie à côté ([[options.gpu_automatic]] au départ) ; désactivé, avec le rendu logiciel de l'ordinateur. Le réglage s'applique au prochain démarrage. Si la fenêtre ne peut pas s'ouvrir ainsi, TraceDraw redémarre avec le rendu logiciel, puis avec OpenGL, et garde celui qui a fonctionné.
- [[options.undo_levels]] : combien d'étapes vous pouvez annuler (150 au
  départ).

## Affichage {#display}

![La page Affichage des options de TraceDraw](../../img/customize-options-app-display.png)

- [[menu.view.proof_colors]] : afficher les couleurs telles qu'elles seront
  imprimées.
- [[options.show_tooltips]] (actif), [[options.hide_bbox_curve_tools]] et
  [[options.node_tracking]] (inactif : actif, il permet à l'outil
  [[tool.pick]] de faire glisser les nœuds d'une courbe sélectionnée).
- [[options.full_screen_preview]] : [[options.show_page_border]].
- [[options.wheel_default_action]] : [[options.wheel_zoom]] (au départ) ou
  [[options.wheel_scroll]].
- [[options.open_view]] : [[options.open_view_page]] (au départ),
  [[options.open_view_objects]], [[options.open_view_width]],
  [[options.open_view_height]], [[options.open_view_actual]] ou
  [[options.open_view_saved]] (la page, l'endroit et le zoom gardés à chaque
  enregistrement d'un fichier TraceDraw).
- [[options.ruler_calibration]] : tenez une règle contre l'écran et changez
  la [[options.screen_resolution]] jusqu'à ce que les graduations
  correspondent, afin que le zoom à 100 % montre les tailles réelles.
  [[options.reset]] remet 96.

## Édition {#edit}

![La page Édition des options de TraceDraw](../../img/customize-options-app-edit.png)

- [[options.constrain_angle]] : le pas de <kbd>Ctrl</kbd> pendant la
  rotation et le dessin (15 degrés au départ).
- [[options.drawing_precision]] : le nombre de décimales dans la barre de
  propriétés et la barre d'état (3 au départ).
- [[options.focus_inactive]] (20 % au départ) et
  [[options.focus_double_click]] ; voir
  [Mode focus](../objects/focus-mode.md).

## Nœuds et poignées {#nodes-and-handles}

![La page Nœuds et poignées des options de TraceDraw](../../img/customize-options-app-nodes-and-handles.png)

- [[options.node_size]] : petite, moyenne ou grande.
- [[options.node_shape]] pour les nœuds [[options.node_cusp]],
  [[options.node_smooth]] et [[options.node_symmetrical]].
- [[options.show_curve_direction]].
- [[options.corner_widgets]].
- [[options.colors]] : la [[options.main_color]] des nœuds et poignées
  sélectionnés et la [[options.secondary_color]].
- [[options.unselected_nodes_filled]].

## Cadre de découpe {#clip-frame}

![La page Cadre de découpe des options de TraceDraw](../../img/customize-options-app-clip-frame.png)

- [[options.auto_center]] : [[options.auto_center_outside]] (au départ),
  [[options.auto_center_always]] ou [[options.auto_center_never]].
- [[options.empty_frame_lines]] (actif).

Voir [Placer des objets dans un cadre](../objects/clip-frames.md).

## Magnétisme {#snapping}

![La page Magnétisme des options de TraceDraw](../../img/customize-options-app-snapping.png)

- [[options.snap_to_objects]], [[options.snap_to_page]],
  [[options.snap_to_guidelines]], [[options.snap_to_grid]], la grille de
  lignes de base et [[options.snap_to_pixels]].
- [[options.snapping_radius]] (10 pixels au départ).
- [[options.show_snap_marks]] et l'[[options.snap_screen_tip]].
- [[options.snap_modes]] : les points auxquels les objets s'aimantent, avec
  [[options.select_all]] et [[options.deselect_all]].

Voir [Magnétisme](../pages-layout/snapping.md).

## Enregistrement {#save}

![La page Enregistrement des options de TraceDraw](../../img/customize-options-app-save.png)

- [[options.backup]] : [[options.backup_before_save]] (actif) conserve le
  fichier précédent sous le nom `backup_of_<nom>`, [[options.backup_to]] le
  [[options.same_folder]] ou un emplacement choisi.
- [[options.auto_backup]] (actif) : [[options.backup_every]] 20
  [[options.minutes]], dans un dossier temporaire ou un emplacement choisi.
  Après un plantage, les sauvegardes sont proposées ; voir
  [Sauvegarde automatique et récupération](../get-started/open-and-save.md#recovery).

## Texte {#text}

![La page Texte des options de TraceDraw](../../img/customize-options-app-text.png)

La page a cinq onglets.

- [[options.display]] : [[options.keyboard_text_increment]] (de combien
  <kbd>Ctrl+8</kbd> et <kbd>Ctrl+2</kbd> agrandissent et réduisent le texte
  sélectionné, 1 point), la [[options.default_font]] et la
  [[options.default_size]] du nouveau texte, et [[menu.text.show_non_printing]].
  - [[options.text_units]] pt, mm, cm, in, px…
- [[options.paragraph_text]] : [[menu.text.show_text_frames]],
  [[menu.text.use_hyphenation]] et les [[options.hyphenation_patterns]]
  (intégrés, ou un dictionnaire de césure de votre langue).
- [[docker.fonts]] : [[options.show_recent_fonts]] en haut des listes de
  polices, le [[options.recent_font_count]] (5, jusqu'à 20) et la
  [[options.font_sample_size]].
- [[options.spelling]] : [[writing.ignore_digits]], [[writing.ignore_capitals]]
  et [[options.add_to_autocorrect]], pour qu'un mot corrigé dans le correcteur
  soit corrigé à la frappe la fois suivante ; et les
  [[options.dictionaries]] qu'il utilise, voir
  [Dictionnaires](../text/writing-tools.md#dictionaries).
- [[dialog.autocorrect]] : ce qui est corrigé à la frappe ; voir
  [Correction automatique](../text/writing-tools.md#autocorrect).
