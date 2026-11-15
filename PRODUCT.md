# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Single static HTML file (inline CSS + JS, no build step). Opened locally from a file or folder.

## Users

One person (the owner) working through every Marvel Cinematic Universe film and TV show before *Avengers: Doomsday* (Dec 2026). Uses it equally on phone (ticking off titles on the couch) and laptop (planning what to watch next).

## Product Purpose

A personal watch planner: the full MCU in release order with checkboxes, time remaining, and a prerequisite flowchart showing what to watch before any given title. Success = always knowing what's next and how much is left.

## Positioning

Not a generic list: it combines release-order progress with a per-title "what must I watch first" graph, and counts only unwatched time.

## Capabilities and Constraints

- Watchlist in release order, grouped by year; tap to mark watched; progress and hours left.
- Option to hide the Netflix Defenders-era shows.
- Prerequisite flowchart per title: must-watch and should-watch links (owner's judgment, not canon), re-centre by tapping a node.
- State saved in localStorage (key `mcu-wl2`); must keep working and keep existing saved progress.
- Runtimes approximate; "~" marks estimates. Doomsday excluded from totals.

## Evidence on Hand

Title data embedded in `index.html`. No official artwork or posters; do not use Marvel logos or studio branding.

## Product Principles

1. "What's next" is always one glance away.
2. Progress should feel rewarding.
3. Reading the prereq chart must be effortless on a phone.

## Brand Commitments

Owner wants it bold and cinematic and explicitly hates the current "generic AI vibecoded" look.
