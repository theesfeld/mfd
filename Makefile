# VGE — pure assembly library (System V AMD64)
# Product is libvge, not a Rust wrapper.
#
#   make            → build/libvge.a build/libvge.so
#   make test       → C smoke test
#   make install    → PREFIX (default ~/.local)
#
# Any language: link -lvge -lm and #include <vge.h>

PREFIX   ?= $(HOME)/.local
CC       ?= cc
AS       ?= as
AR       ?= ar
CFLAGS   ?= -O2 -Wall -Wextra -Iinclude
ASFLAGS  ?= --64
LDFLAGS  ?= -Lbuild -lvge -lm

ASM_SRC  := asm/x86_64/vge.s asm/x86_64/vge_extra.s
ASM_OBJ  := $(patsubst asm/x86_64/%.s,build/%.o,$(ASM_SRC))

.PHONY: all clean test install shared static

all: static shared

static: build/libvge.a
shared: build/libvge.so

build:
	mkdir -p build

build/%.o: asm/x86_64/%.s | build
	$(AS) $(ASFLAGS) -o $@ $<

build/libvge.a: $(ASM_OBJ)
	$(AR) rcs $@ $(ASM_OBJ)
	@echo "built $@"

build/libvge.so: $(ASM_OBJ)
	$(CC) -shared -o $@ $(ASM_OBJ) -lm
	@echo "built $@"

build/smoke: examples/c/smoke.c build/libvge.a
	$(CC) $(CFLAGS) -o $@ examples/c/smoke.c build/libvge.a -lm

test: build/smoke
	./build/smoke

install: all
	install -d $(PREFIX)/lib $(PREFIX)/include
	install -m 644 build/libvge.a $(PREFIX)/lib/
	install -m 755 build/libvge.so $(PREFIX)/lib/
	install -m 644 include/vge.h $(PREFIX)/include/
	@echo "installed to $(PREFIX) — use: -I$(PREFIX)/include -L$(PREFIX)/lib -lvge -lm"

clean:
	rm -rf build
