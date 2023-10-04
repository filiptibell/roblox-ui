<!-- Disable lint that disallows html -->
<!-- markdownlint-disable MD033 -->

<h1 align="center">Rojo Viewer</h1>

<div align="center">
<a href="https://marketplace.visualstudio.com/items?itemName=filiptibell.rojo-viewer">
<img src="https://vsmarketplacebadges.dev/version/filiptibell.rojo-viewer.png"/>
</a>
</div>

<br/>

An extension that brings the Explorer view and more from Roblox Studio into Visual Studio Code.

This extension is currently under heavy development.

---

## Installation

The extension can be installed from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=filiptibell.rojo-viewer).

## Development

The extension can also be compiled and installed locally:

1. Clone the repository
2. Install [Just], [VSCE] and the [VSCode CLI]
3. Run `just install` in the repository to install the extension

[Just]: https://github.com/casey/just
[VSCE]: https://github.com/microsoft/vscode-vsce
[VSCode CLI]: https://code.visualstudio.com/docs/editor/command-line

---

## Project Status

### Missing Features

-   Add support for showing a file picker (`rbxm` and `rbxmx`) and inserting them into instances
-   Implement remaining file tree operations - copy/cut/paste/paste into
-   Implement drag & drop functionality for the file tree

### Current Bugs

-   Fix `.meta` files not being renamed together with main file in rename/delete operations

### Future Plans

-   Improved integration with [Wally]
    -   Button for opening the wally manifest, similar to the one for the rojo manifest
    -   Hover cards for wally link files displaying name, version, desc, and link to wally page
-   Properties panel
    -   Simple text view of properties
    -   Editing properties for simple `.model.json` or `.meta` files
    -   Complex property editing for properties such as colors, vectors
    -   Editing properties for binary/xml files, maybe using [Lune] as a backend?
-   Output panel
    -   Server that can listen for output messages from Roblox Studio
    -   Plugin that sends output messages from Roblox Studio to the extension
    -   Automatically connect to a test session and its output when one starts
    -   Create an output channel/panel in VSCode that forwards received output
    -   Parse output and use sourcemap to create clickable file links
    -   Colorize output messages and stack traces
-   Debugger integration (requires output first)
    -   Press F5 to start debugger and a Roblox Studio testing session
    -   Stopping the debugger by pressing F5 or any of the buttons also stops studio

[Wally]: https://github.com/UpliftGames/wally
[Lune]: https://github.com/filiptibell/lune

---

## Icons and Attribution

-   The [Classic] (also known as Silk) icon pack used in the extension was created by famfamfam and is licensed under [CC BY 4.0]
-   The [Vanilla] version 2.1 icon pack used in the extension was created by Elttob and is licensed under [CC BY-NC 4.0]

[Classic]: https://github.com/legacy-icons/famfamfam-silk
[Vanilla]: https://github.com/Elttob/Vanilla
[CC BY 4.0]: https://creativecommons.org/licenses/by/4.0/
[CC BY-NC 4.0]: https://creativecommons.org/licenses/by-nc/4.0/
