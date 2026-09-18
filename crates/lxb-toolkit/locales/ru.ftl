# What the controls the toolkit draws for an application say: the file
# question, its menus and the notes under its rows. An application's own
# labels are its own catalog's — see docs/localization.md.

## The file question, by what it is asking for.
choose-a-file = Выберите файл
choose-some-files = Выберите файлы
choose-a-folder = Выберите папку
choose-somewhere-to-save = Выберите, куда сохранить
choose-an-image = Выберите изображение
choose-scenery = Выберите фон

## What the button under it does.
open = Открыть
use-this-folder = Использовать эту папку
save-here = Сохранить здесь
choose-this = Выбрать это

## The legend along the foot.
choose = Выбрать
select = Выбрать
approve = Разрешить
options = Параметры
cancel = Отмена

## The rows at the head of a listing.
new-folder = Новая папка
make-a-folder-here = Создать здесь папку
name = Имя
what-the-file-will-be-called = Как будет называться файл
untitled = Без названия
search = Поиск
search-this-folder-by-name = Искать в этой папке по имени
clear-search = Очистить поиск
show-every-item-in-this-folder = Показать всё в этой папке
nothing-chosen-yet = Пока ничего не выбрано
give-the-file-a-name-first = Сначала дайте файлу имя

## The Options menu over a listing.
types = Типы
sort = Сортировка
everything = Всё
hide-hidden-files = Скрыть скрытые файлы
show-hidden-files = Показать скрытые файлы
images = Изображения
images-and-films = Изображения и фильмы

## The orders a listing can be in.
name-a-to-z = По имени (от А до Я)
name-z-to-a = По имени (от Я до А)
size-largest-first = По размеру (сначала крупные)
size-smallest-first = По размеру (сначала мелкие)
file-type = Тип
created-newest-first = По дате создания (сначала новые)
created-oldest-first = По дате создания (сначала старые)
modified-newest-first = По дате изменения (сначала новые)
modified-oldest-first = По дате изменения (сначала старые)

## What a row in a listing says about itself.
folder = Папка
file = Файл
empty = Пусто
modified-date-unknown = Дата изменения неизвестна

## What the question says about itself, above the listing. The two spaces
## are the gap between the word and the answer; the bar is the text cursor.
showing-kinds = Показано  { $kinds }
saving-as = Сохранить как  { $name }
saving-as-typing = Сохранить как  { $name }|
save-as-name = Сохранить как { $name }

## Refusals, in the words the person who pressed the row is owed.
a-folder-needs-a-name = Папке нужно имя
that-is-not-a-name-a-folder-can-have = Папка не может так называться
something-here-is-called-that-already = Здесь уже есть что-то с таким именем
folder-could-not-be-made = Не удалось создать папку: { $why }
this-cannot-be-opened = Это нельзя открыть

## Counts. The number is passed as a number, never as text, so that a
## language selects the right form of the noun.
count-files = { $count ->
    [one] { $count } файл
    [few] { $count } файла
    [many] { $count } файлов
   *[other] { $count } файлов
    }
count-folders = { $count ->
    [one] { $count } папка
    [few] { $count } папки
    [many] { $count } папок
   *[other] { $count } папок
    }
folder-contents = { $folders ->
    [one] { $folders } папка
    [few] { $folders } папки
    [many] { $folders } папок
   *[other] { $folders } папок
    }, { $files ->
    [one] { $files } файл
    [few] { $files } файла
    [many] { $files } файлов
   *[other] { $files } файлов
    }
folder-more-not-shown = { $counted }, ещё { $more } не показано
matching-items = { $count ->
    [one] { $count } подходящий элемент
    [few] { $count } подходящих элемента
    [many] { $count } подходящих элементов
   *[other] { $count } подходящих элементов
    }
files-chosen = { $count ->
    [one] Выбран { $count } файл
    [few] Выбрано { $count } файла
    [many] Выбрано { $count } файлов
   *[other] Выбрано { $count } файлов
    }

## A date a row carries, and the months it is written with. Polish writes
## the month in the genitive, which is why these are not the names on their
## own — see docs/localization.md.
file-date = { $day } { $month } { $year } г.

## The time of day, on whichever of the two clocks Settings > System >
## Clock is set to. The shell draws both in the corner of its start screen,
## which is cut from a closed set of characters — so a language may write
## them with digits, the colon, the slash, the full stop, the space and the
## letters of clock-am and clock-pm, and nothing else.
clock-24-hour = { $hour }:{ PAD2($minute) }
clock-12-hour = { $hour }:{ PAD2($minute) } { $half }
clock-am = AM
clock-pm = PM
month-january = января
month-february = февраля
month-march = марта
month-april = апреля
month-may = мая
month-june = июня
month-july = июля
month-august = августа
month-september = сентября
month-october = октября
month-november = ноября
month-december = декабря
