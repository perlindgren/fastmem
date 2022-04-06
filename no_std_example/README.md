# no_std example

This crate provides a set of small examples for the `fastmem` crate, utilizing the RTIC framework. The examples target the Nucleo 64 STM32F401/411, but should be portable to any ARM Cortex-M architecture.

## examples

- `main.rs` a minimal example using `fastmem` in a sequential context.

- `bench_alloc.rs` benchmarking allocations and reallocations in a sequential context.

- `juggle.rs` benchmarking allocations, drop and re-allocations in a multi tasking setting.

## testing

The examples utilize semihosting for tracing. The `Cortex Debug` launch profiles can be used as follows:

- `Cortex Debug (No ITM)` builds and executes the selected example in Debug/dev mode.

- `Cortex Release (No ITM)` builds end executes the selected example in release mode.

- `Cortex Release (No ITM) trace` builds and executes the selected example in release mode with internal tracing enabled (`--features trace_semihost`).

