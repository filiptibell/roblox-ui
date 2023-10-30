<!-- markdownlint-disable MD033 -->
<!-- markdownlint-disable MD041 -->

<img align="right" width="256" src="assets/icon-256.png" />

<h1 align="center">Roblox UI</h1>

<div align="center">
  <a href="https://github.com/filiptibell/roblox-ui/actions">
    <img src="https://shields.io/endpoint?url=https://badges.readysetplay.io/workflow/filiptibell/roblox-ui/ci.yaml" alt="CI status" />
  </a>
  <a href="https://github.com/filiptibell/roblox-ui/actions">
    <img src="https://shields.io/endpoint?url=https://badges.readysetplay.io/workflow/filiptibell/roblox-ui/release.yaml" alt="Release status" />
  </a>
  <a href="https://github.com/filiptibell/roblox-ui/blob/main/LICENSE.txt">
    <img src="https://img.shields.io/github/license/filiptibell/roblox-ui.svg?label=License&color=informational" alt="Project license" />
  </a>
</div>

<br/>

A frontend for Roblox projects in external editors.

Provides an Explorer view, with future plans for a Properties view and more. <br/>
Check out the [features](#features) section for a full list of features.

## Installation

The UI can be installed as an extension from:

- The [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=filiptibell.roblox-ui) (VSCode)

## Features

- Explorer view for instances, similar to Roblox Studio
- Included community icon packs, as well as the classic icon pack
- Integration with [Wally](https://wally.run) in the explorer view

## TODO

<details>
<summary>Wally</summary>

- Button for opening the wally manifest, similar to the one for the rojo manifest
- Hover cards for wally link files displaying name, version, desc, and link to wally page

</details>

<details>
<summary>Properties</summary>

- Simple text view of properties
- Editing properties for simple `.model.json` or `.meta` files
- Complex property editing for properties such as colors, vectors
- Editing properties for binary/xml files

</details>

<details>
<summary>Output</summary>

- Server that can listen for output messages from Roblox Studio
- Plugin that sends output messages from Roblox Studio to the extension
- Automatically connect to a test session and its output when one starts
- Create an output channel/panel in VSCode that forwards received output
- Parse output and use sourcemap to create clickable file links
- Colorize output messages and stack traces

</details>

<details>
<summary>Debugger</summary>

- Press F5 to start debugger and a Roblox Studio testing session
- Stopping the debugger by pressing F5 or any of the buttons also stops studio

</details>

## Development

The VSCode extension can be compiled and installed locally:

1. Clone the repository
2. Install [Just], [Rust], [VSCE] and the [VSCode CLI]
3. Run `just vscode-install` in the repository to install the extension

[Just]: https://github.com/casey/just
[Rust]: https://www.rust-lang.org/tools/install
[VSCE]: https://github.com/microsoft/vscode-vsce
[VSCode CLI]: https://code.visualstudio.com/docs/editor/command-line

## Icons and Attribution

- The [Classic] (also known as Silk) icon pack used in the extension was created by famfamfam and is licensed under [CC BY 4.0]
- The [Vanilla] version 2.1 icon pack used in the extension was created by Elttob and is licensed under [CC BY-NC 4.0]

[Classic]: https://github.com/legacy-icons/famfamfam-silk
[Vanilla]: https://github.com/Elttob/Vanilla
[CC BY 4.0]: https://creativecommons.org/licenses/by/4.0/
[CC BY-NC 4.0]: https://creativecommons.org/licenses/by-nc/4.0/
