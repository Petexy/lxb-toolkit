# What the controls the toolkit draws for an application say: the file
# question, its menus and the notes under its rows. An application's own
# labels are its own catalog's — see docs/localization.md.

## The file question, by what it is asking for.
choose-a-file = 选择一个文件
choose-some-files = 选择一些文件
choose-a-folder = 选择一个文件夹
choose-somewhere-to-save = 选择保存位置
choose-an-image = 选择一张图片
choose-scenery = 选择一个场景

## What the button under it does.
open = 打开
use-this-folder = 使用此文件夹
save-here = 保存到此处
choose-this = 选择这个

## The legend along the foot.
choose = 选择
select = 选择
approve = 允许
options = 选项
cancel = 取消

## The rows at the head of a listing.
new-folder = 新建文件夹
make-a-folder-here = 在此新建文件夹
name = 名称
what-the-file-will-be-called = 文件的名称
untitled = 无标题
search = 搜索
search-this-folder-by-name = 按名称搜索此文件夹
clear-search = 清除搜索
show-every-item-in-this-folder = 显示此文件夹中的所有项目
nothing-chosen-yet = 尚未选择任何内容
give-the-file-a-name-first = 请先给文件起个名称

## The Options menu over a listing.
types = 类型
sort = 排序
everything = 全部
hide-hidden-files = 隐藏隐藏文件
show-hidden-files = 显示隐藏文件
images = 图片
images-and-films = 图片和影片

## The orders a listing can be in.
name-a-to-z = 名称（A 到 Z）
name-z-to-a = 名称（Z 到 A）
size-largest-first = 大小（最大在前）
size-smallest-first = 大小（最小在前）
file-type = 类型
created-newest-first = 创建时间（最新在前）
created-oldest-first = 创建时间（最旧在前）
modified-newest-first = 修改时间（最新在前）
modified-oldest-first = 修改时间（最旧在前）

## What a row in a listing says about itself.
folder = 文件夹
file = 文件
empty = 空
modified-date-unknown = 修改日期未知

## What the question says about itself, above the listing. The two spaces
## are the gap between the word and the answer; the bar is the text cursor.
showing-kinds = 正在显示  { $kinds }
saving-as = 保存为  { $name }
saving-as-typing = 保存为  { $name }|
save-as-name = 保存为 { $name }

## Refusals, in the words the person who pressed the row is owed.
a-folder-needs-a-name = 文件夹需要一个名称
that-is-not-a-name-a-folder-can-have = 文件夹不能使用这个名称
something-here-is-called-that-already = 这里已有同名的项目
folder-could-not-be-made = 无法创建文件夹：{ $why }
this-cannot-be-opened = 无法打开

## Counts. The number is passed as a number, never as text, so that a
## language selects the right form of the noun.
count-files = { $count } 个文件
count-folders = { $count } 个文件夹
folder-contents = { $folders } 个文件夹，{ $files } 个文件
folder-more-not-shown = { $counted }，另有 { $more } 项未显示
matching-items = { $count } 个匹配项
files-chosen = 已选择 { $count } 个文件

## A date a row carries, and the months it is written with. Polish writes
## the month in the genitive, which is why these are not the names on their
## own — see docs/localization.md.
file-date = { $year }年{ $month }{ $day }日

## The time of day, on whichever of the two clocks Settings > System >
## Clock is set to. The shell draws both in the corner of its start screen,
## which is cut from a closed set of characters — so a language may write
## them with digits, the colon, the slash, the full stop, the space and the
## letters of clock-am and clock-pm, and nothing else.
clock-24-hour = { $hour }:{ PAD2($minute) }
clock-12-hour = { $hour }:{ PAD2($minute) } { $half }
clock-am = AM
clock-pm = PM
month-january = 1月
month-february = 2月
month-march = 3月
month-april = 4月
month-may = 5月
month-june = 6月
month-july = 7月
month-august = 8月
month-september = 9月
month-october = 10月
month-november = 11月
month-december = 12月
