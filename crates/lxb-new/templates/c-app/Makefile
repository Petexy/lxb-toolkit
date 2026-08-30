CFLAGS += -std=c11 -Wall -Wextra $(shell pkg-config --cflags lxb-app)
LDLIBS += $(shell pkg-config --libs lxb-app)

{{SLUG}}: src/main.c
	$(CC) $(CFLAGS) $< $(LDLIBS) -o $@

clean:
	rm -f {{SLUG}}

.PHONY: clean
