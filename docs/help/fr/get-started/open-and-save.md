# Ouvrir et enregistrer des dessins

> *Traduction automatique : cette page peut être en retard sur la version anglaise.*

TraceDraw enregistre les dessins dans son propre format, `.tdraw`, qui
conserve tout, et peut aussi enregistrer des fichiers `.cdr`. Il ouvre de
nombreux autres formats ; voir
[Formats de fichier pris en charge](../file-formats/supported-formats.md).

## Ouvrir un dessin {#open}

1. Choisissez [[menu.file > menu.file.open]] (<kbd>Ctrl+O</kbd>), ou cliquez
   sur Ouvrir dans la barre d'outils standard.
2. Choisissez le fichier et confirmez.

Le dessin s'ouvre dans un nouvel onglet. Pour ouvrir un dessin utilisé
récemment, utilisez [[menu.file > menu.file.open_recent]], la flèche du
bouton Ouvrir ou la liste de l'écran d'accueil ;
[[menu.file.clear_recent]] vide la liste. Pour retirer un seul dessin de la
liste de l'écran d'accueil, pointez-le et cliquez sur le x à sa droite, ou
cliquez dessus avec le bouton droit et choisissez [[welcome.remove_recent]] ;
le fichier lui-même reste à sa place.
[[menu.file > menu.file.import]] (<kbd>Ctrl+I</kbd>) ajoute plutôt le contenu
d'un fichier au dessin ouvert ; voir
[Importer des fichiers](../file-formats/import.md).

Vous pouvez aussi faire glisser des fichiers depuis votre gestionnaire de
fichiers sur TraceDraw. Un dessin (un fichier `.tdraw` ou `.cdr`, par
exemple) lâché sur la barre de titre, les menus, les barres d'outils ou les
onglets de documents s'ouvre dans son propre onglet ; lâché sur la fenêtre
de dessin, il est importé dans le dessin ouvert. Le cadre et l'étiquette
affichés pendant le glissement indiquent ce qui va se passer. Là où le
système n'indique pas où se trouve le pointeur (Linux, macOS), les dessins
s'ouvrent dans un onglet et les images sont importées.

La façon dont un dessin s'affiche d'abord (la page entière, ses objets, la
largeur de la page, la taille réelle, ou la page, la position et le zoom
qu'il avait à l'enregistrement) se règle avec [[options.open_view]], dans la
page [[options.display]] de
[[menu.tools > menu.tools.options > menu.tools.options_app]].

### Messages après l'ouverture {#open-notes}

Quand certains objets d'un fichier ouvert n'ont pas pu être lus, ou que le
dessin utilise des polices qui ne sont pas installées sur votre ordinateur,
un message l'indique après l'ouverture du fichier :

- Pour les objets qui n'ont pas pu être lus, il en donne le nombre. Un
  bouton **Signaler** ouvre un nouveau ticket pour le fichier dans le suivi
  des problèmes du projet, afin que le problème puisse être corrigé.
- Pour les polices manquantes, la boîte de dialogue [[missing_fonts.title]]
  les liste, chacune avec la police installée qui la dessine, et vous laisse
  en choisir une autre pour cette session ou la remplacer dans le dessin.
  Voir [Polices manquantes](../text/missing-fonts.md).

Cochez **Ne plus afficher ce message** pour ne plus voir ces messages. Vous
pouvez les réactiver dans la page [[options.warnings]] de
[[menu.tools > menu.tools.options > menu.tools.options_global]].

## Enregistrer un dessin {#save}

Choisissez [[menu.file > menu.file.save]] (<kbd>Ctrl+S</kbd>) pour écrire le
dessin dans son fichier. Un nouveau dessin, ou un dessin ouvert depuis un
format que TraceDraw sait seulement lire, n'a pas encore de fichier :
Enregistrer ouvre alors la boîte de dialogue [[dialog.save_drawing]].

## Enregistrer sous {#save-as}

[[menu.file > menu.file.save_as]] (<kbd>Ctrl+Shift+S</kbd>) enregistre le
dessin sous un autre nom, dans un autre dossier ou dans un autre format.

![La boîte de dialogue Enregistrer le dessin](../../img/get-started-open-and-save-save-as.png)

Pour enregistrer un dessin sous un nouveau nom :

1. Choisissez le dossier. La liste de gauche contient des raccourcis vers
   [[dialog.place_home]], [[dialog.place_desktop]],
   [[dialog.place_documents]], [[dialog.place_downloads]] et
   [[dialog.place_pictures]], ainsi que vers les lecteurs.
   [[dialog.up_folder]] remonte au dossier parent ; vous pouvez aussi taper
   un dossier dans la barre de chemin. Double-cliquez sur un
   dossier de la liste pour l'ouvrir ; cliquez sur un fichier pour reprendre
   son nom.
2. Tapez le [[dialog.file_name]]. L'extension du type choisi est ajoutée.
3. Choisissez [[dialog.save_as_type]] : TraceDraw (`*.tdraw`) ou
   [[file.cdr_files]].
4. Choisissez la [[dialog.cdr_version]]. Pour les fichiers TraceDraw, c'est
   la version du format, la plus récente en premier. Pour les fichiers
   `.cdr`, c'est la version du fichier, de 27 à 7 ; un programme plus
   ancien ne peut ouvrir qu'une version aussi ancienne que lui ou plus
   ancienne. La version choisie reste attachée au dessin : Enregistrer
   l'écrit donc de nouveau sans rien demander.
5. Choisissez ce qui accompagne le dessin dans le fichier :
   - Pour les fichiers `.cdr` à partir de la version 16,
     [[dialog.embed_color_profile_none]] stocke le profil CMJN du dessin dans
     le fichier.
   - [[dialog.embed_fonts]] (décoché au départ) stocke les polices utilisées
     par le texte, quand leur licence le permet, dans les fichiers TraceDraw
     et dans les fichiers `.cdr` à partir de la version 16. La case indique
     ce que les polices ajoutent au fichier. Cochez-la pour les dessins qui
     vont sur un ordinateur qui n'a peut-être pas les polices.
   - Pour les fichiers TraceDraw sur ordinateur, [[dialog.save_package]]
     enregistre le dessin dans un dossier à son nom avec les images liées,
     les fichiers de police et un rapport, et [[package.pdf]] ajoute un PDF.
     Voir [Créer un paquet](../file-formats/package.md).

   Ces choix sont conservés pour la fois suivante.
6. Si vous le souhaitez, remplissez [[dialog.title]], [[dialog.subject]] et
   une [[dialog.rating]] (cliquez de nouveau sur la dernière étoile allumée
   pour l'enlever). Ce sont les propriétés du dessin, affichées aussi dans
   [Propriétés du document](../pages-layout/document-properties.md).
7. Cliquez sur [[dialog.save]] ou appuyez sur <kbd>Entrée</kbd>. Si le
   fichier existe, TraceDraw demande s'il faut le [[dialog.replace]].

La boîte de dialogue s'ouvre dans le dossier du dessin, sinon dans le
dernier dossier où vous avez enregistré, sinon dans votre dossier de
documents. Un dessin ouvert depuis un fichier `.cdr` ou un autre format
s'ouvre dans le dossier de ce fichier, avec son nom : enregistrer dans le
même format le remplace donc.

Dans la liste des fichiers, [[dialog.new_folder]] crée un dossier et vous
laisse taper son nom aussitôt ; <kbd>F2</kbd> ou un clic droit renomme un
dossier ou un fichier ; taper des lettres saute aux éléments qui commencent
par elles, et <kbd>Entrée</kbd> ouvre le dossier choisi. Un dossier tapé dans
la barre de chemin est pris sans appuyer sur <kbd>Entrée</kbd>.

> **Remarque :** Dans la version pour navigateur, chaque enregistrement est
> un téléchargement : la boîte de dialogue conserve le nom, le type, la
> version et les propriétés, et le navigateur place le fichier dans son
> dossier de téléchargements.

Autres façons d'enregistrer : [[menu.file > menu.file.save_as_template]]
enregistre un modèle (voir [Modèles](../templates-styles/templates.md)),
[[menu.file.export]] écrit d'autres formats (voir
[Exporter](../file-formats/export.md)), [[menu.file.publish_to_pdf]] écrit
un PDF (voir [Publier au format PDF](../file-formats/publish-pdf.md)),
[[menu.file.send_to]] enregistre un PDF sur votre bureau ou dans votre
dossier de documents, ou le joint à un e-mail, [[menu.file.package]] réunit
le dessin, ses images et ses polices dans un dossier (voir
[Créer un paquet](../file-formats/package.md)), et [[menu.file.github]]
conserve les dessins dans un dépôt GitHub (voir [GitHub](github.md)).
[[menu.file.revert]] revient à la dernière version enregistrée.

## Fermer des dessins {#closing}

Choisissez [[menu.file > menu.file.close]] (<kbd>Ctrl+F4</kbd>), cliquez sur
le bouton de fermeture de l'onglet du dessin, ou cliquez sur l'onglet avec
le bouton du milieu. [[menu.file.close_all]] ferme tous les dessins, et
[[menu.file.exit]] (<kbd>Alt+F4</kbd>) ferme TraceDraw.

Quand un dessin a des modifications non enregistrées, TraceDraw demande
d'abord s'il faut les enregistrer : [[dialog.yes]] enregistre (en demandant
un nom si le dessin n'a pas encore de fichier), [[dialog.no]] ferme sans
enregistrer et [[dialog.cancel]] laisse le dessin ouvert. Quand plusieurs
dessins ont des modifications, il pose la question pour chacun ; Annuler
arrête.

![La question sur les modifications non enregistrées](../../img/get-started-open-and-save-closing.png)

## Sauvegarde automatique et récupération {#recovery}

Sur le bureau, TraceDraw conserve des sauvegardes de vos dessins ouverts
tant que la sauvegarde automatique est active (la page [[options.save]] de
[[menu.tools > menu.tools.options > menu.tools.options_app]] règle la
fréquence et l'emplacement ; toutes les 20 minutes au départ). Fermer
TraceDraw normalement les supprime.

Si TraceDraw s'est arrêté sans se fermer, par exemple après une coupure de
courant, il propose les sauvegardes au démarrage suivant :
[[dialog.recover]] ouvre chacune comme un dessin non enregistré
(Enregistrer demande alors où l'enregistrer), et [[dialog.discard]] laisse
les sauvegardes dans leur dossier. Deux fenêtres TraceDraw ouvertes en même
temps ne se proposent jamais les dessins l'une de l'autre.
