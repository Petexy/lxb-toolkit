# What the controls the toolkit draws for an application say: the file
# question, its menus and the notes under its rows. An application's own
# labels are its own catalog's — see docs/localization.md.

## The file question, by what it is asking for.
choose-a-file = Choisir un fichier
choose-some-files = Choisir des fichiers
choose-a-folder = Choisir un dossier
choose-somewhere-to-save = Choisir où enregistrer
choose-an-image = Choisir une image
choose-scenery = Choisir un décor

## What the button under it does.
open = Ouvrir
use-this-folder = Utiliser ce dossier
save-here = Enregistrer ici
choose-this = Choisir ceci

## The legend along the foot.
choose = Choisir
select = Sélectionner
approve = Valider
options = Options
cancel = Annuler

## The rows at the head of a listing.
new-folder = Nouveau dossier
make-a-folder-here = Créer un dossier ici
name = Nom
what-the-file-will-be-called = Le nom que portera le fichier
untitled = Sans titre
search = Rechercher
search-this-folder-by-name = Chercher dans ce dossier par nom
clear-search = Effacer la recherche
show-every-item-in-this-folder = Afficher tout le contenu de ce dossier
nothing-chosen-yet = Rien de sélectionné pour l'instant
give-the-file-a-name-first = Donnez d'abord un nom au fichier

## The Options menu over a listing.
types = Types
sort = Trier
everything = Tout
hide-hidden-files = Masquer les fichiers cachés
show-hidden-files = Afficher les fichiers cachés
images = Images
images-and-films = Images et films

## The orders a listing can be in.
name-a-to-z = Nom (de A à Z)
name-z-to-a = Nom (de Z à A)
size-largest-first = Taille (de la plus grande à la plus petite)
size-smallest-first = Taille (de la plus petite à la plus grande)
file-type = Type
created-newest-first = Créé (du plus récent au plus ancien)
created-oldest-first = Créé (du plus ancien au plus récent)
modified-newest-first = Modifié (du plus récent au plus ancien)
modified-oldest-first = Modifié (du plus ancien au plus récent)

## What a row in a listing says about itself.
folder = Dossier
file = Fichier
empty = Vide
modified-date-unknown = Date de modification inconnue

## What the question says about itself, above the listing. The two spaces
## are the gap between the word and the answer; the bar is the text cursor.
showing-kinds = Affiche  { $kinds }
saving-as = Enregistrer sous  { $name }
saving-as-typing = Enregistrer sous  { $name }|
save-as-name = Enregistrer sous { $name }

## Refusals, in the words the person who pressed the row is owed.
a-folder-needs-a-name = Un dossier a besoin d'un nom
that-is-not-a-name-a-folder-can-have = Un dossier ne peut pas porter ce nom
something-here-is-called-that-already = Quelque chose ici porte déjà ce nom
folder-could-not-be-made = Le dossier n'a pas pu être créé : { $why }
this-cannot-be-opened = Ceci ne peut pas être ouvert

## Counts. The number is passed as a number, never as text, so that a
## language selects the right form of the noun.
count-files = { $count ->
    [one] { $count } fichier
   *[other] { $count } fichiers
    }
count-folders = { $count ->
    [one] { $count } dossier
   *[other] { $count } dossiers
    }
folder-contents = { $folders ->
    [one] { $folders } dossier
   *[other] { $folders } dossiers
    }, { $files ->
    [one] { $files } fichier
   *[other] { $files } fichiers
    }
folder-more-not-shown = { $counted }, { $more } de plus non affichés
matching-items = { $count ->
    [one] { $count } élément correspondant
   *[other] { $count } éléments correspondants
    }
files-chosen = { $count ->
    [one] { $count } fichier sélectionné
   *[other] { $count } fichiers sélectionnés
    }

## A date a row carries, and the months it is written with. Polish writes
## the month in the genitive, which is why these are not the names on their
## own — see docs/localization.md.
file-date = { $day } { $month } { $year }

## The time of day, on whichever of the two clocks Settings > System >
## Clock is set to. The shell draws both in the corner of its start screen,
## which is cut from a closed set of characters — so a language may write
## them with digits, the colon, the slash, the full stop, the space and the
## letters of clock-am and clock-pm, and nothing else.
clock-24-hour = { $hour }:{ PAD2($minute) }
clock-12-hour = { $hour }:{ PAD2($minute) } { $half }
clock-am = AM
clock-pm = PM
month-january = janvier
month-february = février
month-march = mars
month-april = avril
month-may = mai
month-june = juin
month-july = juillet
month-august = août
month-september = septembre
month-october = octobre
month-november = novembre
month-december = décembre
