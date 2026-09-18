# What the controls the toolkit draws for an application say: the file
# question, its menus and the notes under its rows. An application's own
# labels are its own catalog's — see docs/localization.md.

## The file question, by what it is asking for.
choose-a-file = Escolher um arquivo
choose-some-files = Escolher alguns arquivos
choose-a-folder = Escolher uma pasta
choose-somewhere-to-save = Escolher onde salvar
choose-an-image = Escolher uma imagem
choose-scenery = Escolher um cenário

## What the button under it does.
open = Abrir
use-this-folder = Usar esta pasta
save-here = Salvar aqui
choose-this = Escolher este

## The legend along the foot.
choose = Escolher
select = Selecionar
approve = Aprovar
options = Opções
cancel = Cancelar

## The rows at the head of a listing.
new-folder = Nova pasta
make-a-folder-here = Criar uma pasta aqui
name = Nome
what-the-file-will-be-called = Como o arquivo vai se chamar
untitled = Sem título
search = Pesquisar
search-this-folder-by-name = Pesquisar nesta pasta por nome
clear-search = Limpar pesquisa
show-every-item-in-this-folder = Mostrar tudo o que há nesta pasta
nothing-chosen-yet = Nada escolhido ainda
give-the-file-a-name-first = Dê um nome ao arquivo primeiro

## The Options menu over a listing.
types = Tipos
sort = Ordenar
everything = Tudo
hide-hidden-files = Ocultar arquivos ocultos
show-hidden-files = Mostrar arquivos ocultos
images = Imagens
images-and-films = Imagens e filmes

## The orders a listing can be in.
name-a-to-z = Nome (A a Z)
name-z-to-a = Nome (Z a A)
size-largest-first = Tamanho (maiores primeiro)
size-smallest-first = Tamanho (menores primeiro)
file-type = Tipo
created-newest-first = Criação (mais recentes primeiro)
created-oldest-first = Criação (mais antigos primeiro)
modified-newest-first = Modificação (mais recentes primeiro)
modified-oldest-first = Modificação (mais antigos primeiro)

## What a row in a listing says about itself.
folder = Pasta
file = Arquivo
empty = Vazia
modified-date-unknown = Data de modificação desconhecida

## What the question says about itself, above the listing. The two spaces
## are the gap between the word and the answer; the bar is the text cursor.
showing-kinds = Mostrando  { $kinds }
saving-as = Salvando como  { $name }
saving-as-typing = Salvando como  { $name }|
save-as-name = Salvar como { $name }

## Refusals, in the words the person who pressed the row is owed.
a-folder-needs-a-name = Uma pasta precisa de um nome
that-is-not-a-name-a-folder-can-have = Uma pasta não pode ter esse nome
something-here-is-called-that-already = Algo aqui já se chama assim
folder-could-not-be-made = A pasta não pôde ser criada: { $why }
this-cannot-be-opened = Isto não pode ser aberto

## Counts. The number is passed as a number, never as text, so that a
## language selects the right form of the noun.
count-files = { $count ->
    [one] { $count } arquivo
   *[other] { $count } arquivos
    }
count-folders = { $count ->
    [one] { $count } pasta
   *[other] { $count } pastas
    }
folder-contents = { $folders ->
    [one] { $folders } pasta
   *[other] { $folders } pastas
    }, { $files ->
    [one] { $files } arquivo
   *[other] { $files } arquivos
    }
folder-more-not-shown = { $counted }, mais { $more } não mostrados
matching-items = { $count ->
    [one] { $count } item correspondente
   *[other] { $count } itens correspondentes
    }
files-chosen = { $count ->
    [one] { $count } arquivo escolhido
   *[other] { $count } arquivos escolhidos
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
month-january = janeiro
month-february = fevereiro
month-march = março
month-april = abril
month-may = maio
month-june = junho
month-july = julho
month-august = agosto
month-september = setembro
month-october = outubro
month-november = novembro
month-december = dezembro
