---
layout: ../../layouts/Guide.astro
title: Status
description: Version and stability, supported platforms, minimum Rust and what is planned.
---

## Version and stability

iced-cube is still on 0.0.x, and every release may include breaking changes: builder methods and variant names can move as more components arrive. Cargo treats each 0.0.x release as incompatible with the last, so a requirement such as `iced-cube = "0.0.2"` stays on that release until you change it. Read the changelog before you upgrade.

Each release targets one iced version, currently 0.14.

## Supported platforms

- **Linux** (X11 and Wayland) and **Windows** are built and tested on every change.
- **The browser** (WebAssembly with WebGL) runs every example on this site.
- **macOS** should work wherever iced does, but it is not tested yet.

Screen reader support is limited to what iced itself offers.

## Minimum Rust

Rust 1.88 or newer, which is what iced 0.14 needs. iced-cube uses the 2024 edition. Only the current stable toolchain is tested.

## Planned components

These are next on the roadmap, roughly in order:

- File tree
- Property grid
- Virtual list and virtual tree
- Log viewer
- JSON viewer
- Inspector
- Document tabs
- App shell

## Changelog

Every release is listed in the [changelog](https://github.com/sam-ruff/iced-cube/blob/main/CHANGELOG.md), generated from the commit history.
