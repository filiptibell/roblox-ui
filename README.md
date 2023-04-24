# Rojo Explorer

This extension is currently under development and as such is not yet published to the extension marketplace.

Once the extension reaches a more usable state it will be published.

## Installation

1. Clone the repository
2. Install [Aftman](https://github.com/LPGhatguy/aftman), [VSCE](https://github.com/microsoft/vscode-vsce) and the [VSCode CLI](https://code.visualstudio.com/docs/editor/command-line)
3. Run `aftman install` in the repository
4. Run `just install` in the repository to install the extension

## Plans

### TODO

-   Implement remaining file tree operations - copy/cut/paste/paste into
-   Implement drag & drop functionality for the file tree
-   Implement support for Rojo's `.meta` files in rename/delete operations
-   Implement support for adding arbitrary classes using `.model.json` files
-   Automatically download & store explorer icons in a cache instead of hardcoding
-   Parse the Rojo project file that the user has selected to discover root files/dirs
-   Fix folders & files being able to be created outside of project root directories

### Future

-   Properties panel to see the current properties of instances
-   Properties editing in the properties panel for `.model.json` or `.meta` files
-   Properties editing in the properties panel for binary/xml files, maybe using [Lune](https://github.com/filiptibell/lune) as a backend?
