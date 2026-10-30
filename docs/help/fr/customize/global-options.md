# Options globales

[[menu.tools > menu.tools.options > menu.tools.options_global]] contient les
réglages qui s'appliquent à tout le programme : la langue, les valeurs par
défaut d'impression et d'exportation, la vectorisation, les messages et
l'emplacement des fichiers.

## Général {#general}

![La page Général des options globales](../../img/customize-global-options-general.png)

- [[options.language]] : la langue de l'interface. Cette Aide s'ouvre dans
  la même langue.
- [[options.check_updates]] (actif) : voir
  [Rechercher des mises à jour](../welcome/about-and-updates.md#updates).
- [[options.agents]] : [[options.mcp_window]] permet aux assistants d'IA de
  cet ordinateur de modifier le dessin ouvert ; voir
  [Scripts](scripts.md#mcp).
- [[options.reset_all]] : voir
  [Réinitialiser les paramètres](reset-settings.md).

## Impression {#printing}

![La page Impression des options globales](../../img/customize-global-options-printing.png)

[[options.print_starts_with]] : [[dialog.fit_to_page]],
[[dialog.separations]], [[dialog.print_bleed]], [[dialog.crop_marks]],
[[dialog.registration_marks]], [[print.color_bars]], [[print.densitometer]],
[[print.page_info_mark]] (tous inactifs), la [[print.sheet]],
[[print.landscape]] et la [[print.gutter]] (5 mm).
[[options.print_remember]] conserve les choix de la dernière impression.
Sous [[options.print_checks]], [[options.print_preflight]] (actif). Voir
[Imprimer un dessin](../printing/print.md).

## Formats de fichier {#file-formats}

![La page Formats de fichier des options globales](../../img/customize-global-options-file-formats.png)

- [[options.export_starts_with]] : le [[dialog.format]] (SVG), la
  [[dialog.resolution]] des formats bitmap (300 ppp), la
  [[dialog.quality]] du JPEG et du WebP (90),
  [[dialog.transparent_background]] (actif), la [[dialog.pdf_standard]], la
  [[dialog.output_condition]] et le [[dialog.bleed]].
  [[options.export_remember]] conserve les choix de la dernière exportation.
- [[options.import_section]] : [[options.import_place]]
  [[options.import_as_in_file]] (au départ) ou
  [[options.import_view_center]].
- [[options.cdr_save_section]] : la [[dialog.cdr_version]] dans laquelle les
  nouveaux dessins sont enregistrés en `.cdr`.

## Vectorisation {#bitmap-tracing}

![La page Vectorisation des options globales](../../img/customize-global-options-bitmap-tracing.png)

[[options.quick_trace_with]] l'un des préréglages (Clipart au départ).
[[options.trace_starts_with]] [[dialog.remove_background]] (actif) et
[[dialog.delete_original]] (inactif). Voir
[Vectoriser des images bitmap](../bitmaps/tracing.md).

## Avertissements {#warnings}

![La page Avertissements des options globales](../../img/customize-global-options-warnings.png)

[[options.warnings_show]] [[options.warn_unread_objects]],
[[options.warn_missing_fonts]] et [[options.warn_shelf_remove]] (tous
actifs). Chaque message a aussi une case qui le désactive ; cochez l'option
ici pour le revoir.

## Emplacements des fichiers {#file-locations}

![La page Emplacements des fichiers des options globales](../../img/customize-global-options-file-locations.png)

Indique où sont conservées les préférences et les sauvegardes automatiques.
Les préférences se trouvent dans un dossier `tracedraw-settings` à côté du
programme quand il existe (une copie portable), sinon à l'emplacement prévu
par votre système pour les paramètres des programmes ; la version pour
navigateur les conserve dans le navigateur.
