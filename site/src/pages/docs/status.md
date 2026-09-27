---
layout: ../../layouts/Guide.astro
title: Status
description: Version and stability, supported platforms, minimum Rust and what is planned.
---

## Version and stability

iced-cube is at 0.0.1. Expect breaking changes between releases while it is on 0.x: builder methods and variant names may still move as more components arrive. Breaking changes bump the minor version, so a caret requirement such as `iced-cube = "0.1"` will not pick them up by surprise once 0.1 is out.

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

- Popover
- Dialog
- Dropdown menu
- Context menu
- Combobox
- Command
- Sidebar
- Tree
- Data table
- Split pane

## Changelog

Every release is listed in the [changelog](https://github.com/sam-ruff/iced-cube/blob/main/CHANGELOG.md), generated from the commit history.
