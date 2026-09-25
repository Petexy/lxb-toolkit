# Build the C examples against the toolkit built in this repository.
#
#     cargo build --release      # from the repository root, first
#     make hello && ./hello      # the greeting, in a window
#     make tour && ./tour        # the whole language, eight pages of it
#     make && ./swatch           # the same, as a picture, on the processor
#
# Against an installed toolkit instead, drop the two paths:
#     cc swatch.c $$(pkg-config --cflags --libs lxb-toolkit) -o swatch

ROOT := ../..
CFLAGS += -std=c11 -Wall -Wextra -I$(ROOT)/crates/lxb-toolkit-ffi/include
LDFLAGS += -L$(ROOT)/target/release -Wl,-rpath,$(abspath $(ROOT)/target/release)
LDLIBS += -llxb_toolkit

# A struct in either header that changes shape changes every program built
# against it, so each one is rebuilt when a header it includes moves.
TOOLKIT_H := $(ROOT)/crates/lxb-toolkit-ffi/include/lxb_toolkit.h
APP_H := $(ROOT)/crates/lxb-app-ffi/include/lxb_app.h

swatch: swatch.c $(TOOLKIT_H)
	$(CC) $(CFLAGS) $< $(LDFLAGS) $(LDLIBS) -o $@

# The short form: no window system, no painter, no font service — the app
# library brings all three. This is what an application actually looks like.
APP_CFLAGS := -I$(ROOT)/crates/lxb-app-ffi/include
hello: hello.c $(TOOLKIT_H) $(APP_H)
	$(CC) $(CFLAGS) $(APP_CFLAGS) $< $(LDFLAGS) -llxb_app $(LDLIBS) -o $@

# The eight-page tour, drawn by the same renderer. It is the transcription of
# examples/python/tour.py, and the two are compared frame for frame.
tour: tour.c $(TOOLKIT_H) $(APP_H)
	$(CC) $(CFLAGS) $(APP_CFLAGS) -DLXB_TOUR_FILES_DEFAULT=\"$(abspath ../tour-files)\" \
		$< $(LDFLAGS) -llxb_app $(LDLIBS) -lm -o $@

# The static archive, for a binary with nothing to find at run time. It carries
# the marks, the recordings and the faces with it, so it is not small.
swatch-static: swatch.c $(TOOLKIT_H)
	$(CC) $(CFLAGS) $< $(ROOT)/target/release/liblxb_toolkit.a -lm -lpthread -ldl -o $@

clean:
	rm -f swatch swatch-static swatch.svg hello

.PHONY: clean
