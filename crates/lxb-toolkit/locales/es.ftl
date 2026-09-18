# What the controls the toolkit draws for an application say: the file
# question, its menus and the notes under its rows. An application's own
# labels are its own catalog's — see docs/localization.md.

## The file question, by what it is asking for.
choose-a-file = Elegir un archivo
choose-some-files = Elegir unos archivos
choose-a-folder = Elegir una carpeta
choose-somewhere-to-save = Elegir dónde guardar
choose-an-image = Elegir una imagen
choose-scenery = Elegir un escenario

## What the button under it does.
open = Abrir
use-this-folder = Usar esta carpeta
save-here = Guardar aquí
choose-this = Elegir esto

## The legend along the foot.
choose = Elegir
select = Seleccionar
approve = Aceptar
options = Opciones
cancel = Cancelar

## The rows at the head of a listing.
new-folder = Carpeta nueva
make-a-folder-here = Crear una carpeta aquí
name = Nombre
what-the-file-will-be-called = Cómo se llamará el archivo
untitled = Sin título
search = Buscar
search-this-folder-by-name = Buscar en esta carpeta por nombre
clear-search = Borrar la búsqueda
show-every-item-in-this-folder = Mostrar todo el contenido de esta carpeta
nothing-chosen-yet = Todavía no se ha elegido nada
give-the-file-a-name-first = Dele antes un nombre al archivo

## The Options menu over a listing.
types = Tipos
sort = Ordenar
everything = Todo
hide-hidden-files = Ocultar los archivos ocultos
show-hidden-files = Mostrar los archivos ocultos
images = Imágenes
images-and-films = Imágenes y vídeos

## The orders a listing can be in.
name-a-to-z = Nombre (de la A a la Z)
name-z-to-a = Nombre (de la Z a la A)
size-largest-first = Tamaño (del mayor al menor)
size-smallest-first = Tamaño (del menor al mayor)
file-type = Tipo
created-newest-first = Creado (del más nuevo al más antiguo)
created-oldest-first = Creado (del más antiguo al más nuevo)
modified-newest-first = Modificado (del más nuevo al más antiguo)
modified-oldest-first = Modificado (del más antiguo al más nuevo)

## What a row in a listing says about itself.
folder = Carpeta
file = Archivo
empty = Vacía
modified-date-unknown = Fecha de modificación desconocida

## What the question says about itself, above the listing. The two spaces
## are the gap between the word and the answer; the bar is the text cursor.
showing-kinds = Mostrando  { $kinds }
saving-as = Guardando como  { $name }
saving-as-typing = Guardando como  { $name }|
save-as-name = Guardar como { $name }

## Refusals, in the words the person who pressed the row is owed.
a-folder-needs-a-name = Una carpeta necesita un nombre
that-is-not-a-name-a-folder-can-have = Una carpeta no puede llamarse así
something-here-is-called-that-already = Algo de aquí ya se llama así
folder-could-not-be-made = No se pudo crear la carpeta: { $why }
this-cannot-be-opened = Esto no se puede abrir

## Counts. The number is passed as a number, never as text, so that a
## language selects the right form of the noun.
count-files = { $count ->
    [one] { $count } archivo
   *[other] { $count } archivos
    }
count-folders = { $count ->
    [one] { $count } carpeta
   *[other] { $count } carpetas
    }
folder-contents = { $folders ->
    [one] { $folders } carpeta
   *[other] { $folders } carpetas
    }, { $files ->
    [one] { $files } archivo
   *[other] { $files } archivos
    }
folder-more-not-shown = { $counted }, { $more } más sin mostrar
matching-items = { $count ->
    [one] { $count } elemento coincidente
   *[other] { $count } elementos coincidentes
    }
files-chosen = { $count ->
    [one] { $count } archivo seleccionado
   *[other] { $count } archivos seleccionados
    }

## A date a row carries, and the months it is written with. Polish writes
## the month in the genitive, which is why these are not the names on their
## own — see docs/localization.md.
file-date = { $day } de { $month } de { $year }

## The time of day, on whichever of the two clocks Settings > System >
## Clock is set to. The shell draws both in the corner of its start screen,
## which is cut from a closed set of characters — so a language may write
## them with digits, the colon, the slash, the full stop, the space and the
## letters of clock-am and clock-pm, and nothing else.
clock-24-hour = { $hour }:{ PAD2($minute) }
clock-12-hour = { $hour }:{ PAD2($minute) } { $half }
clock-am = AM
clock-pm = PM
month-january = enero
month-february = febrero
month-march = marzo
month-april = abril
month-may = mayo
month-june = junio
month-july = julio
month-august = agosto
month-september = septiembre
month-october = octubre
month-november = noviembre
month-december = diciembre
