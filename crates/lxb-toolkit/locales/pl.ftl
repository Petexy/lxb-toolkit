# Polskie tłumaczenie kontrolek rysowanych przez toolkit. Klucze i nazwy
# zmiennych są takie same jak w en.ftl; liczby są przekazywane jako liczby,
# więc forma rzeczownika wybierana jest tutaj.

## Pytanie o plik, zależnie od tego, o co pyta.
choose-a-file = Wybierz plik
choose-some-files = Wybierz pliki
choose-a-folder = Wybierz folder
choose-somewhere-to-save = Wybierz miejsce zapisu
choose-an-image = Wybierz obraz
choose-scenery = Wybierz tło

## Co robi przycisk pod nim.
open = Otwórz
use-this-folder = Użyj tego folderu
save-here = Zapisz tutaj
choose-this = Wybierz to

## Legenda przy dolnej krawędzi.
choose = Zaznacz
select = Wybierz
approve = Zatwierdź
options = Opcje
cancel = Anuluj

## Wiersze na początku listy.
new-folder = Nowy folder
make-a-folder-here = Utwórz tutaj folder
name = Nazwa
what-the-file-will-be-called = Nazwa zapisywanego pliku
untitled = Bez nazwy
search = Szukaj
search-this-folder-by-name = Szukaj w tym folderze według nazwy
clear-search = Wyczyść wyszukiwanie
show-every-item-in-this-folder = Pokaż wszystkie elementy tego folderu
nothing-chosen-yet = Nic jeszcze nie wybrano
give-the-file-a-name-first = Najpierw nadaj plikowi nazwę

## Menu Opcje nad listą.
types = Typy
sort = Sortuj
everything = Wszystko
hide-hidden-files = Ukryj ukryte pliki
show-hidden-files = Pokaż ukryte pliki
images = Obrazy
images-and-films = Obrazy i filmy

## Porządek, w jakim lista jest ułożona.
name-a-to-z = Nazwa (od A do Z)
name-z-to-a = Nazwa (od Z do A)
size-largest-first = Rozmiar (od największych)
size-smallest-first = Rozmiar (od najmniejszych)
file-type = Typ
created-newest-first = Utworzenie (od najnowszych)
created-oldest-first = Utworzenie (od najstarszych)
modified-newest-first = Modyfikacja (od najnowszych)
modified-oldest-first = Modyfikacja (od najstarszych)

## Co wiersz listy mówi o sobie.
folder = Folder
file = Plik
empty = Pusty
modified-date-unknown = Nieznana data modyfikacji

## Co pytanie mówi o sobie, nad listą. Dwie spacje to odstęp między słowem
## a odpowiedzią, a pionowa kreska to kursor tekstu.
showing-kinds = Widok:  { $kinds }
saving-as = Zapis jako  { $name }
saving-as-typing = Zapis jako  { $name }|
save-as-name = Zapisz jako { $name }

## Odmowy, powiedziane osobie, która nacisnęła wiersz.
a-folder-needs-a-name = Folder musi mieć nazwę
that-is-not-a-name-a-folder-can-have = To nie jest poprawna nazwa folderu
something-here-is-called-that-already = Element o tej nazwie już tu istnieje
folder-could-not-be-made = Nie udało się utworzyć folderu: { $why }
this-cannot-be-opened = Nie można tego otworzyć

## Liczby rzeczy.
count-files = { $count ->
    [one] { $count } plik
    [few] { $count } pliki
    [many] { $count } plików
   *[other] { $count } pliku
    }
count-folders = { $count ->
    [one] { $count } folder
    [few] { $count } foldery
    [many] { $count } folderów
   *[other] { $count } folderu
    }
folder-contents = { $folders ->
    [one] { $folders } folder
    [few] { $folders } foldery
    [many] { $folders } folderów
   *[other] { $folders } folderu
    }, { $files ->
    [one] { $files } plik
    [few] { $files } pliki
    [many] { $files } plików
   *[other] { $files } pliku
    }
folder-more-not-shown = { $counted }, nie pokazano jeszcze { $more ->
    [one] { $more } elementu
   *[other] { $more } elementów
    }
matching-items = { $count ->
    [one] { $count } pasujący element
    [few] { $count } pasujące elementy
    [many] { $count } pasujących elementów
   *[other] { $count } pasujących elementów
    }
files-chosen = { $count ->
    [one] Wybrano { $count } plik
    [few] Wybrano { $count } pliki
    [many] Wybrano { $count } plików
   *[other] Wybrano { $count } plików
    }

## Data przy wierszu i miesiące, którymi jest pisana — po polsku w
## dopełniaczu, bo data brzmi „1 stycznia 1970".
file-date = { $day } { $month } { $year }

## Godzina — w tym z dwóch zegarów, który wskazuje Ustawienia > System >
## Zegar. Powłoka rysuje oba w rogu ekranu startowego, wyciętym z zamkniętego
## zbioru znaków, więc wolno tu użyć tylko cyfr, dwukropka, ukośnika, kropki,
## spacji i liter z clock-am oraz clock-pm.
clock-24-hour = { $hour }:{ PAD2($minute) }
clock-12-hour = { $hour }:{ PAD2($minute) } { $half }
clock-am = AM
clock-pm = PM
month-january = stycznia
month-february = lutego
month-march = marca
month-april = kwietnia
month-may = maja
month-june = czerwca
month-july = lipca
month-august = sierpnia
month-september = września
month-october = października
month-november = listopada
month-december = grudnia
