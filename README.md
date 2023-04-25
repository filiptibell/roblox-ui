# Rojo Viewer

An extension that brings the Explorer view and more from Roblox Studio into Visual Studio Code.

This extension is currently under development heavy development.

## Installation

1. Clone the repository
2. Install [Aftman](https://github.com/LPGhatguy/aftman), [VSCE](https://github.com/microsoft/vscode-vsce) and the [VSCode CLI](https://code.visualstudio.com/docs/editor/command-line)
3. Run `aftman install` in the repository
4. Run `just install` in the repository to install the extension

## Project Status

### Unimplemented

#### Features

-   Add customizable ignore globs for which instances to show (default to ignoring `**/Index/**`)
-   Add support for showing a file picker (`rbxm` and `rbxmx`) and inserting them into instances
-   Add support for inserting services if the root of the explorer is a DataModel
-   Add support for adding arbitrary classes using `.model.json` files, improved Insert Object dialog
-   Implement remaining file tree operations - copy/cut/paste/paste into
-   Implement drag & drop functionality for the file tree

#### Improvements

-   Automatically download & store explorer icons in a cache instead of hardcoding

#### Bugs

-   Fix `.meta` files not being renamed together with main file in rename/delete operations

### Future Plans

-   Integration with [Wally](https://github.com/UpliftGames/wally)
    -   Button for opening the wally manifest, similar to the one for the rojo manifest
    -   Hovering over the root of a tree for info about version, description, ...
    -   Special "package" icon for projects that are publishable and have a script as the root
-   Properties panel
    -   Simple text view of properties
    -   Editing properties for simple `.model.json` or `.meta` files
    -   Complex property editing for properties such as colors, vectors
    -   Editing properties for binary/xml files, maybe using [Lune](https://github.com/filiptibell/lune) as a backend?
