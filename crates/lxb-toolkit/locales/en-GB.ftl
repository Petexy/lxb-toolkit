# What the controls the toolkit draws for an application say: the file
# question, its menus and the notes under its rows. An application's own
# labels are its own catalog's — see docs/localization.md.

## The file question, by what it is asking for.
choose-a-file = Choose a file
choose-some-files = Choose some files
choose-a-folder = Choose a folder
choose-somewhere-to-save = Choose somewhere to save
choose-an-image = Choose an image
choose-scenery = Choose scenery

## What the button under it does.
open = Open
use-this-folder = Use this folder
save-here = Save here
choose-this = Choose this

## The legend along the foot.
choose = Choose
select = Select
approve = Approve
options = Options
cancel = Cancel

## The rows at the head of a listing.
new-folder = New folder
make-a-folder-here = Make a folder here
name = Name
what-the-file-will-be-called = What the file will be called
untitled = Untitled
search = Search
search-this-folder-by-name = Search this folder by name
clear-search = Clear search
show-every-item-in-this-folder = Show every item in this folder
nothing-chosen-yet = Nothing chosen yet
give-the-file-a-name-first = Give the file a name first

## The Options menu over a listing.
types = Types
sort = Sort
everything = Everything
hide-hidden-files = Hide hidden files
show-hidden-files = Show hidden files
images = Images
images-and-films = Images and films

## The orders a listing can be in.
name-a-to-z = Name (A to Z)
name-z-to-a = Name (Z to A)
size-largest-first = Size (largest first)
size-smallest-first = Size (smallest first)
file-type = Type
created-newest-first = Created (newest first)
created-oldest-first = Created (oldest first)
modified-newest-first = Modified (newest first)
modified-oldest-first = Modified (oldest first)

## What a row in a listing says about itself.
folder = Folder
file = File
empty = Empty
modified-date-unknown = Modified date unknown

## What the question says about itself, above the listing. The two spaces
## are the gap between the word and the answer; the bar is the text cursor.
showing-kinds = Showing  { $kinds }
saving-as = Saving as  { $name }
saving-as-typing = Saving as  { $name }|
save-as-name = Save as { $name }

## Refusals, in the words the person who pressed the row is owed.
a-folder-needs-a-name = A folder needs a name
that-is-not-a-name-a-folder-can-have = That is not a name a folder can have
something-here-is-called-that-already = Something here is called that already
folder-could-not-be-made = The folder could not be made: { $why }
this-cannot-be-opened = This cannot be opened

## Counts. The number is passed as a number, never as text, so that a
## language selects the right form of the noun.
count-files = { $count ->
    [one] { $count } file
   *[other] { $count } files
    }
count-folders = { $count ->
    [one] { $count } folder
   *[other] { $count } folders
    }
folder-contents = { $folders ->
    [one] { $folders } folder
   *[other] { $folders } folders
    }, { $files ->
    [one] { $files } file
   *[other] { $files } files
    }
folder-more-not-shown = { $counted }, { $more } more not shown
matching-items = { $count ->
    [one] { $count } matching item
   *[other] { $count } matching items
    }
files-chosen = { $count ->
    [one] { $count } file chosen
   *[other] { $count } files chosen
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
month-january = January
month-february = February
month-march = March
month-april = April
month-may = May
month-june = June
month-july = July
month-august = August
month-september = September
month-october = October
month-november = November
month-december = December
