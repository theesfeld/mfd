# =============================================================================
# VGE — PURE ASSEMBLY LIBRARY
# =============================================================================
# Source of the product: asm/x86_64/*.s ONLY.
# No C. No Rust. No libc in the library.
#
#   make            → build/libvge.a  build/libvge.so
#   make test       → pure-asm smoke (no C runtime)
#   make install    → PREFIX (default ~/.local)
#
# Any language that can call the System V AMD64 C ABI can load this:
#   #include "vge.h"
#   -lvge
# =============================================================================

PREFIX  ?= $(HOME)/.local
AS      ?= as
LD      ?= ld
AR      ?= ar
CC      ?= cc

ASFLAGS ?= --64
ASM_SRC := asm/x86_64/vge.s asm/x86_64/vge_extra.s asm/x86_64/vge_aa.s
ASM_OBJ := $(patsubst asm/x86_64/%.s,build/%.o,$(ASM_SRC))

.PHONY: all clean test install static shared

all: static shared

static: build/libvge.a
shared: build/libvge.so

build:
	mkdir -p build

build/%.o: asm/x86_64/%.s | build
	$(AS) $(ASFLAGS) -o $@ $<

build/libvge.a: $(ASM_OBJ)
	$(AR) rcs $@ $(ASM_OBJ)
	@echo "OK  $@"

# Shared object: pure asm, no -lc -lm
build/libvge.so: $(ASM_OBJ)
	$(LD) -shared -o $@ $(ASM_OBJ)
	@echo "OK  $@"

# Pure assembly smoke — no C, no Rust, no libc
build/smoke_asm.o: examples/asm/smoke.s | build
	$(AS) $(ASFLAGS) -o $@ $<

build/smoke_asm: build/smoke_asm.o $(ASM_OBJ)
	$(LD) -o $@ build/smoke_asm.o $(ASM_OBJ)

# include AA object in default ASM_OBJ via ASM_SRC
	@echo "OK  $@"

test: build/smoke_asm
	./build/smoke_asm
	@echo "OK  pure-asm smoke exit 0"

install: all
	install -d $(PREFIX)/lib $(PREFIX)/include
	install -m 644 build/libvge.a $(PREFIX)/lib/
	install -m 755 build/libvge.so $(PREFIX)/lib/
	install -m 644 include/vge.h $(PREFIX)/include/
	@echo "installed libvge → $(PREFIX)"

clean:
	rm -rf build
