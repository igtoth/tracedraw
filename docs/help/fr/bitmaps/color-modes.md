# Changer le mode de couleur des images bitmap

> *Traduction automatique : cette page peut être en retard sur la version anglaise.*

[[menu.bitmaps > menu.bitmaps.mode]] convertit les images bitmap
sélectionnées dans un autre mode de couleur. Chaque image sélectionnée change
en une seule étape d'annulation, et la transparence est conservée. Les modes
qui ont des réglages ouvrent une boîte de dialogue avec des aperçus avant et
après ; les réglages sont dans un panneau à droite et, dessous,
[[dialog.reset]] les remet, tandis que les deux flèches annulent et
rétablissent les modifications faites dans la boîte de dialogue.

| Mode | Résultat |
|---|---|
| [[menu.bitmaps.mode_bw]] | noir et blanc uniquement (voir ci-dessous) |
| [[menu.bitmaps.mode_grayscale]] | niveaux de gris |
| [[menu.bitmaps.mode_duotone]] | une à quatre encres (voir ci-dessous) |
| [[menu.bitmaps.mode_paletted]] | 256 couleurs au plus (voir ci-dessous) |
| [[menu.bitmaps.mode_rgb]] | le mode de l'image devient RVB ; les pixels restent tels quels |
| [[menu.bitmaps.mode_lab]] | couleur Lab |
| [[menu.bitmaps.mode_cmyk]] | le mode de l'image devient CMJN et ses pixels montrent les couleurs que les encres peuvent imprimer, calculées avec les profils de gestion des couleurs |

Le menu coche [[menu.bitmaps.mode_rgb]] ou [[menu.bitmaps.mode_cmyk]] quand toutes les images sélectionnées sont dans ce mode, et la barre d'état indique le mode d'une image sélectionnée, par exemple « Bitmap 600 x 400 px (CMYK) ». Une image CMJN d'un fichier `.cdr` s'ouvre en mode CMJN, et les fichiers `.tdraw` gardent le mode. [[menu.tools > menu.tools.convert_colors]] règle de la même façon le mode de chaque image qu'il convertit (voir [Convertir les couleurs](../color/convert-colors.md)).

## Noir et blanc {#black-and-white}

![La boîte de dialogue Noir et blanc](../../img/bitmaps-color-modes-black-and-white.png)

Choisissez la méthode de conversion :

- **Dessin au trait** : blanc là où le gris atteint ou dépasse le **seuil**
  (128 au départ), noir ailleurs ;
- **Ordonné** : un motif régulier de points ;
- **Demi-teinte** (au départ) : une trame de points ronds, de lignes ou de
  points carrés selon un **angle** (45 degrés) et un nombre de **lignes par
  pouce** (60) ;
- **Distribution de cardinalité** : une texture d'aspect aléatoire ;
- **Jarvis**, **Stucki** et **Floyd-Steinberg** : diffusion d'erreur, les
  résultats les plus doux pour les photos.

**Intensité** (de 0 à 100, 50 au départ) rend le résultat plus clair ou plus
foncé.

## Bichromie {#duotone}

![La boîte de dialogue Bichromie](../../img/bitmaps-color-modes-duotone.png)

Une bichromie imprime une image en niveaux de gris avec quelques encres, ce
qui donne des tons riches avec peu de couleurs.

1. Choisissez le type : Monochromie, Bichromie (au départ), Trichromie ou
   Quadrichromie, pour une à quatre encres.
2. Cliquez sur une encre pour changer sa couleur.
3. Modifiez sa courbe : l'axe horizontal est le niveau de gris (le noir à
   gauche), l'axe vertical la quantité de cette encre imprimée. Faites
   glisser dans un espace vide pour ajouter un point, faites glisser un point
   pour le déplacer, double-cliquez dessus ou faites-le glisser hors de la
   grille pour le supprimer. **Tout afficher** dessine aussi les courbes des
   autres encres.
4. **Enregistrer** et **Charger** conservent les encres et les courbes dans
   un fichier.

## Palette {#paletted}

![La boîte de dialogue Palette](../../img/bitmaps-color-modes-paletted.png)

- **Palette** : Couleurs uniformes, VGA standard (16 couleurs), Adaptative et
  Optimisée (les couleurs propres à l'image, réduites au nombre de
  **couleurs** que vous réglez, jusqu'à 256), Niveaux de gris, Système ou
  Personnalisée (la palette du document).
- **Tramage** : Aucun, Ordonné, Jarvis, Stucki ou Floyd-Steinberg, avec
  l'intensité du tramage.

La palette obtenue s'affiche sous les réglages.

## Colorer des images bitmap noir et blanc {#monochrome}

Quand seules des images bitmap à deux couleurs sont sélectionnées, cliquez
sur une couleur de la palette pour colorer leur fond, et cliquez avec le
bouton droit pour colorer leur premier plan.
