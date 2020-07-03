# riscv-sbi-rt

[![Actions Status](https://github.com/rcore-os/riscv-sbi-rt/workflows/CI/badge.svg)](https://github.com/rcore-os/riscv-sbi-rt/actions)

A mininal runtime / startup for Supervisor Binary Interface (SBI) on RISC-V.

## Features

- [x] Minimal entry runtime & pre-init
- [x] Hanlding traps (interrupts and exceptions)
- [x] Friendly macros and compile time checks
- [x] Preparation for frame and page system

Todo:

- [ ] Support for switching between contexts
- [ ] Prepare for user mode, support for system calls
- [ ] Proper support for No-RTOS without paging system

## Example

Dependencies:

- Rust toolchain
- QEMU v4.1.0

Just open [example](./example) directory and `make run`!
