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
2. Install [Aftman](https://github.com/LPGhatguy/aftman), [VSCE](https://github.com/microsoft/vscode-vsce) and the [VSCode CLI](https://code.visualstudio.com/docs/editor/command-line)
3. Run `aftman install` in the repository
4. Run `just install` in the repository to install the extension

---

## Project Status

### Unimplemented

#### Features

-   Add support for inserting services if the root of the explorer is a DataModel
-   Add support for adding arbitrary classes using `.model.json` files, improved Insert Object dialog
-   Add support for showing a file picker (`rbxm` and `rbxmx`) and inserting them into instances
-   Implement remaining file tree operations - copy/cut/paste/paste into
-   Implement drag & drop functionality for the file tree

#### Improvements

-   Automatically download & store explorer icons in a cache instead of hardcoding

#### Bugs

-   Fix open file / insert object ordering in context menu not being consistent with regular file explorer
-   Fix `.meta` files not being renamed together with main file in rename/delete operations

### Future Plans

-   Integration with [Wally](https://github.com/UpliftGames/wally)
    -   Button for opening the wally manifest, similar to the one for the rojo manifest
    -   Hover cards for wally link files displaying name, version, desc, and link to wally page
    -   Special "package" icon for wally link files
-   Properties panel
    -   Simple text view of properties
    -   Editing properties for simple `.model.json` or `.meta` files
    -   Complex property editing for properties such as colors, vectors
    -   Editing properties for binary/xml files, maybe using [Lune](https://github.com/filiptibell/lune) as a backend?
