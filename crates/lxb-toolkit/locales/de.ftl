# What the controls the toolkit draws for an application say: the file
# question, its menus and the notes under its rows. An application's own
# labels are its own catalog's — see docs/localization.md.

## The file question, by what it is asking for.
choose-a-file = Datei wählen
choose-some-files = Dateien wählen
choose-a-folder = Ordner wählen
choose-somewhere-to-save = Speicherort wählen
choose-an-image = Bild wählen
choose-scenery = Hintergrund wählen

## What the button under it does.
open = Öffnen
use-this-folder = Diesen Ordner verwenden
save-here = Hier speichern
choose-this = Dieses wählen

## The legend along the foot.
choose = Auswählen
select = Auswählen
approve = Zulassen
options = Optionen
cancel = Abbrechen

## The rows at the head of a listing.
new-folder = Neuer Ordner
make-a-folder-here = Hier einen Ordner anlegen
name = Name
what-the-file-will-be-called = Wie die Datei heißen wird
untitled = Ohne Titel
search = Suchen
search-this-folder-by-name = Diesen Ordner nach Namen durchsuchen
clear-search = Suche leeren
show-every-item-in-this-folder = Alles in diesem Ordner anzeigen
nothing-chosen-yet = Noch nichts gewählt
give-the-file-a-name-first = Geben Sie der Datei zuerst einen Namen

## The Options menu over a listing.
types = Typen
sort = Sortieren
everything = Alles
hide-hidden-files = Versteckte Dateien ausblenden
show-hidden-files = Versteckte Dateien anzeigen
images = Bilder
images-and-films = Bilder und Filme

## The orders a listing can be in.
name-a-to-z = Name (A bis Z)
name-z-to-a = Name (Z bis A)
size-largest-first = Größe (größte zuerst)
size-smallest-first = Größe (kleinste zuerst)
file-type = Typ
created-newest-first = Erstellt (neueste zuerst)
created-oldest-first = Erstellt (älteste zuerst)
modified-newest-first = Geändert (neueste zuerst)
modified-oldest-first = Geändert (älteste zuerst)

## What a row in a listing says about itself.
folder = Ordner
file = Datei
empty = Leer
modified-date-unknown = Änderungsdatum unbekannt

## What the question says about itself, above the listing. The two spaces
## are the gap between the word and the answer; the bar is the text cursor.
showing-kinds = Angezeigt:  { $kinds }
saving-as = Speichern als  { $name }
saving-as-typing = Speichern als  { $name }|
save-as-name = Speichern als { $name }

## Refusals, in the words the person who pressed the row is owed.
a-folder-needs-a-name = Ein Ordner braucht einen Namen
that-is-not-a-name-a-folder-can-have = So kann ein Ordner nicht heißen
something-here-is-called-that-already = Etwas hier heißt schon so
folder-could-not-be-made = Der Ordner konnte nicht angelegt werden: { $why }
this-cannot-be-opened = Das lässt sich nicht öffnen

## Counts. The number is passed as a number, never as text, so that a
## language selects the right form of the noun.
count-files = { $count ->
    [one] { $count } Datei
   *[other] { $count } Dateien
    }
count-folders = { $count ->
    [one] { $count } Ordner
   *[other] { $count } Ordner
    }
folder-contents = { $folders ->
    [one] { $folders } Ordner
   *[other] { $folders } Ordner
    }, { $files ->
    [one] { $files } Datei
   *[other] { $files } Dateien
    }
folder-more-not-shown = { $counted }, { $more } weitere nicht angezeigt
matching-items = { $count ->
    [one] { $count } passendes Element
   *[other] { $count } passende Elemente
    }
files-chosen = { $count ->
    [one] { $count } Datei gewählt
   *[other] { $count } Dateien gewählt
    }

## A date a row carries, and the months it is written with. Polish writes
## the month in the genitive, which is why these are not the names on their
## own — see docs/localization.md.
file-date = { $day }. { $month } { $year }

## The time of day, on whichever of the two clocks Settings > System >
## Clock is set to. The shell draws both in the corner of its start screen,
## which is cut from a closed set of characters — so a language may write
## them with digits, the colon, the slash, the full stop, the space and the
## letters of clock-am and clock-pm, and nothing else.
clock-24-hour = { $hour }:{ PAD2($minute) }
clock-12-hour = { $hour }:{ PAD2($minute) } { $half }
clock-am = AM
clock-pm = PM
month-january = Januar
month-february = Februar
month-march = März
month-april = April
month-may = Mai
month-june = Juni
month-july = Juli
month-august = August
month-september = September
month-october = Oktober
month-november = November
month-december = Dezember
