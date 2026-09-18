# English, as America writes it.
#
# An overlay rather than a catalog: every message not written here is answered
# out of `en-GB.ftl`, which is the English this toolkit is written in and the
# one every other language falls back to. What belongs here is only what the
# two Englishes really disagree about, and a message copied across unchanged is
# a message that has to be edited twice from the day it is copied.
# `Catalog::validate` fails a file that does either.
#
# See docs/localization.md, and `i18n::LANGUAGES`.

# The month goes first, and the year is fenced off by a comma.
file-date = { $month } { $day }, { $year }
