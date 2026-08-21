# Build the C examples against the toolkit built in this repository.
#
#     cargo build --release      # from the repository root, first
#     make hello && ./hello      # the greeting, in a window
#     make tour && ./tour        # the whole language, seven pages of it
#     make && ./swatch           # the same, as a picture, on the processor
#
# Against an installed toolkit instead, drop the two paths:
#     cc swatch.c $$(pkg-config --cflags --libs lxb-toolkit) -o swatch

ROOT := ../..
CFLAGS += -std=c11 -Wall -Wextra -I$(ROOT)/crates/lxb-toolkit-ffi/include
LDFLAGS += -L$(ROOT)/target/release -Wl,-rpath,$(abspath $(ROOT)/target/release)
LDLIBS += -llxb_toolkit

swatch: swatch.c
	$(CC) $(CFLAGS) $< $(LDFLAGS) $(LDLIBS) -o $@

# The short form: no window system, no painter, no font service — the app
# library brings all three. This is what an application actually looks like.
APP_CFLAGS := -I$(ROOT)/crates/lxb-app-ffi/include
hello: hello.c
	$(CC) $(CFLAGS) $(APP_CFLAGS) $< $(LDFLAGS) -llxb_app $(LDLIBS) -o $@

# The seven-page tour, drawn by the same renderer. It is the transcription of
# examples/python/tour.py, and the two are compared frame for frame.
tour: tour.c
	$(CC) $(CFLAGS) $(APP_CFLAGS) $< $(LDFLAGS) -llxb_app $(LDLIBS) -lm -o $@

# The static archive, for a binary with nothing to find at run time. It carries
# the marks, the recordings and the faces with it, so it is not small.
swatch-static: swatch.c
	$(CC) $(CFLAGS) $< $(ROOT)/target/release/liblxb_toolkit.a -lm -lpthread -ldl -o $@

clean:
	rm -f swatch swatch-static swatch.svg hello

.PHONY: clean
