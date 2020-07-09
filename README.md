# riscv-sbi-rt

[![Actions Status](https://github.com/rcore-os/riscv-sbi-rt/workflows/CI/badge.svg)](https://github.com/rcore-os/riscv-sbi-rt/actions)

A mininal runtime / startup for Supervisor Binary Interface (SBI) on RISC-V.

## Features

- [x] Minimal entry runtime & pre-init
- [x] Handling traps (interrupts and exceptions)
- [x] Friendly macros and compile time checks
- [x] Preparation for frame and page system
- [x] Support for switching between contexts
- [x] Prepare for user mode, support for system calls

Todo:

- [ ] Proper support for RTOS without paging system
- [ ] Code and design pattern cleanup

## Example

Dependencies:

- Rust toolchain
- QEMU v4.1.0

Just open [example](./example) directory and `make run`!
