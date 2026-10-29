# Copier des couleurs et des attributs avec les pipettes

Les outils pipette prennent une couleur, ou tout un ensemble d'attributs,
sur un objet et les donnent à d'autres. Les deux fonctionnent en deux temps,
affichés dans la barre de propriétés : **sélectionner** (prélever un
échantillon) et **appliquer** (peindre avec). Après un prélèvement, l'outil
passe en mode appliquer ; cliquez sur le bouton de sélection pour prélever
de nouveau. <kbd>Esc</kbd> abandonne l'échantillon et revient à l'outil
[[tool.pick]].

Le pointeur est une pipette quand l'outil échantillonne et un pot de peinture
quand il applique. Les images donnent leur couleur même dans un groupe ou un
cadre.

## Pipette de couleur {#color-eyedropper}

1. Choisissez l'outil [[tool.color_eyedropper]].
2. Cliquez sur un objet pour prendre sa couleur : la couleur d'une surface
   uniforme, la première couleur d'un dégradé ou la couleur du contour. Sur
   une image bitmap, il prend la couleur des pixels sous le pointeur, en
   moyenne sur la taille d'échantillon réglée dans la barre de propriétés
   (1 sur 1, 2 sur 2 ou 5 sur 5 pixels). <kbd>Shift</kbd>+clic prend d'abord
   la couleur du contour.
3. Cliquez sur des objets pour les remplir de cette couleur ;
   <kbd>Shift</kbd>+clic pour colorer leur contour (un objet sans contour en
   reçoit un fin). Cliquer à l'intérieur d'une forme fermée vide la remplit.

La barre de propriétés affiche la couleur prélevée.

Pendant que vous déplacez le pointeur pour prendre une couleur, une petite
case à côté montre la couleur en dessous : un échantillon, le nom de la
couleur si c'est une couleur de palette ou une encre d'accompagnement, ses
valeurs dans le mode de couleur du dessin (CMJN ou RVB) et sa valeur
hexadécimale. La pipette d'un sélecteur de couleur montre la même case.

## Pipette d'attributs {#attributes-eyedropper}

L'outil [[tool.attributes_eyedropper]] copie plusieurs propriétés à la fois
d'un objet vers d'autres.

1. Choisissez l'outil [[tool.attributes_eyedropper]].
2. Dans la barre de propriétés, choisissez ce qu'il faut copier :
   - **Propriétés** : contour, surface et texte (la police, la taille et le
     style ; le texte lui-même est conservé). Toutes actives au départ.
   - **Transformations** : taille, rotation et position. Inactives au
     départ.
   - **Effets** : transparence, ombre portée et effets dynamiques. Inactifs
     au départ.
3. Cliquez sur l'objet source.
4. Cliquez sur chaque objet de destination. <kbd>Shift</kbd>+clic prélève
   plutôt un autre objet.

Chaque clic est une étape d'annulation.
