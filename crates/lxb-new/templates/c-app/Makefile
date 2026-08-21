# {{TITLE}}
#
#     make && ./{{SLUG}}
#
# One library, found through pkg-config. It brings the whole stack with it —
# the renderer, a window system, the controllers and an audio device — so there
# is no toolkit, no painter and no event loop to wire up here.
CFLAGS += -std=c11 -Wall -Wextra $(shell pkg-config --cflags lxb-app)
LDLIBS += $(shell pkg-config --libs lxb-app)

{{SLUG}}: src/main.c
	$(CC) $(CFLAGS) $< $(LDLIBS) -o $@

clean:
	rm -f {{SLUG}}

.PHONY: clean
