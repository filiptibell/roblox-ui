# Rojo Explorer

## About

This extension is currently under development.

To test the extension, follow these steps:

1. Clone the repository
2. Install [Aftman](https://github.com/LPGhatguy/aftman) and [VSCE](https://github.com/microsoft/vscode-vsce)
3. Run `aftman install` in the repository
4. Run `just package` in the repository
5. A VSCode extension file should now have been created at `bin/rojo-explorer-0.0.1.vsix`
6. Install the extension by right clicking and selecting the install option on the above file while in VSCode

## TODO

-   [ ] Implement remaining file tree operations - copy/cut/paste/paste into
-   [ ] Implement support for Rojo's `.meta` files in rename/delete operations
-   [ ] Implement support for adding arbitrary classes using `.model.json` files
-   [ ] Automatically download & store icons instead of hardcoding
-   [x] Automatically download & store the Roblox API dump to get rid of some more hardcoding
-   [ ] Fix folders & files being able to be created outside of project root directories
-   [x] Fix ordering of instances, make it consistent with Roblox Studio
