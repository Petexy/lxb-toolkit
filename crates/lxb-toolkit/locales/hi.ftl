# What the controls the toolkit draws for an application say: the file
# question, its menus and the notes under its rows. An application's own
# labels are its own catalog's — see docs/localization.md.

## The file question, by what it is asking for.
choose-a-file = एक फ़ाइल चुनें
choose-some-files = कुछ फ़ाइलें चुनें
choose-a-folder = एक फ़ोल्डर चुनें
choose-somewhere-to-save = सहेजने की जगह चुनें
choose-an-image = एक छवि चुनें
choose-scenery = एक दृश्य चुनें

## What the button under it does.
open = खोलें
use-this-folder = इस फ़ोल्डर का उपयोग करें
save-here = यहाँ सहेजें
choose-this = इसे चुनें

## The legend along the foot.
choose = चुनें
select = चुनें
approve = स्वीकृति दें
options = विकल्प
cancel = रद्द करें

## The rows at the head of a listing.
new-folder = नया फ़ोल्डर
make-a-folder-here = यहाँ एक फ़ोल्डर बनाएँ
name = नाम
what-the-file-will-be-called = फ़ाइल का नाम क्या होगा
untitled = बिना शीर्षक
search = खोजें
search-this-folder-by-name = इस फ़ोल्डर में नाम से खोजें
clear-search = खोज साफ़ करें
show-every-item-in-this-folder = इस फ़ोल्डर की हर चीज़ दिखाएँ
nothing-chosen-yet = अभी कुछ नहीं चुना गया
give-the-file-a-name-first = पहले फ़ाइल को एक नाम दें

## The Options menu over a listing.
types = प्रकार
sort = क्रमबद्ध करें
everything = सब कुछ
hide-hidden-files = छिपी फ़ाइलें छिपाएँ
show-hidden-files = छिपी फ़ाइलें दिखाएँ
images = छवियाँ
images-and-films = छवियाँ और फ़िल्में

## The orders a listing can be in.
name-a-to-z = नाम (A से Z)
name-z-to-a = नाम (Z से A)
size-largest-first = आकार (सबसे बड़ा पहले)
size-smallest-first = आकार (सबसे छोटा पहले)
file-type = प्रकार
created-newest-first = बनाया गया (नवीनतम पहले)
created-oldest-first = बनाया गया (सबसे पुराना पहले)
modified-newest-first = संशोधित (नवीनतम पहले)
modified-oldest-first = संशोधित (सबसे पुराना पहले)

## What a row in a listing says about itself.
folder = फ़ोल्डर
file = फ़ाइल
empty = खाली
modified-date-unknown = संशोधन की तारीख अज्ञात

## What the question says about itself, above the listing. The two spaces
## are the gap between the word and the answer; the bar is the text cursor.
showing-kinds = दिखाया जा रहा है  { $kinds }
saving-as = इस नाम से सहेजें  { $name }
saving-as-typing = इस नाम से सहेजें  { $name }|
save-as-name = { $name } के रूप में सहेजें

## Refusals, in the words the person who pressed the row is owed.
a-folder-needs-a-name = फ़ोल्डर को एक नाम चाहिए
that-is-not-a-name-a-folder-can-have = फ़ोल्डर का ऐसा नाम नहीं हो सकता
something-here-is-called-that-already = यहाँ पहले से इस नाम की कोई चीज़ है
folder-could-not-be-made = फ़ोल्डर बनाया नहीं जा सका: { $why }
this-cannot-be-opened = इसे खोला नहीं जा सकता

## Counts. The number is passed as a number, never as text, so that a
## language selects the right form of the noun.
count-files = { $count ->
    [one] { $count } फ़ाइल
   *[other] { $count } फ़ाइलें
    }
count-folders = { $count ->
    [one] { $count } फ़ोल्डर
   *[other] { $count } फ़ोल्डर
    }
folder-contents = { $folders ->
    [one] { $folders } फ़ोल्डर
   *[other] { $folders } फ़ोल्डर
    }, { $files ->
    [one] { $files } फ़ाइल
   *[other] { $files } फ़ाइलें
    }
folder-more-not-shown = { $counted }, { $more } और नहीं दिखाए गए
matching-items = { $count ->
    [one] { $count } मेल खाता आइटम
   *[other] { $count } मेल खाते आइटम
    }
files-chosen = { $count ->
    [one] { $count } फ़ाइल चुनी गई
   *[other] { $count } फ़ाइलें चुनी गईं
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
month-january = जनवरी
month-february = फ़रवरी
month-march = मार्च
month-april = अप्रैल
month-may = मई
month-june = जून
month-july = जुलाई
month-august = अगस्त
month-september = सितंबर
month-october = अक्तूबर
month-november = नवंबर
month-december = दिसंबर
