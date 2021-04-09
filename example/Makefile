arch ?= riscv64
target := $(arch)imac-unknown-none-elf
mode := debug
kernel := target/$(target)/$(mode)/opensbi-rt-example

sysroot := $(shell rustc --print sysroot)
objdump := $(shell find $(sysroot) -name llvm-objdump) --arch-name=$(arch)
objcopy := $(shell find $(sysroot) -name llvm-objcopy)

ifeq ($(mode), release)
	BUILD_ARGS += --release
endif

.PHONY: kernel build clean qemu run env

build: $(bin)

env:
	rustup component add llvm-tools-preview rustfmt
	rustup target add $(target)

kernel:
	cargo build $(BUILD_ARGS) --target $(target)

asm:
	$(objdump) -d $(kernel) | less

sym:
	$(objdump) -t $(kernel) | less

header:
	$(objdump) -x $(kernel) | less

clean:
	cargo clean

qemu: kernel
	qemu-system-$(arch) \
		-machine virt \
		-nographic \
		-bios default \
		-kernel $(kernel)

run: build qemu
