<!-- markdownlint-disable MD023 -->
<!-- markdownlint-disable MD033 -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Fixed

- Fixed spaces and periods in instance names not being allowed for insert & rename operations.

## `0.3.3` - July 24th, 2024

### Added

- Added support for custom icon packs! These work exactly like custom icon packs in Roblox Studio do, and have the same setting, which is called `roblox-ui.explorer.customIconDir` in the VSCode extension. Note that you may need to restart VSCode after changing this setting.

## `0.3.2` - July 24th, 2024

### Fixed

- Fixed explorer flickering issue / constant refreshing introduced in `0.3.1`.

## `0.3.1` - July 23rd, 2024

### Fixed

- Fixed `explorer.showDataModel` not working properly after version `0.3.0`.
- Fixed instances having inaccurate hierarchies and/or classes during extension startup.

## `0.3.0` - December 31st, 2023

### Added

- Added a "Quick Open" menu to fuzzy search and open corresponding(s) file for available instances. This can be used as a command (with optional keybind) or by clicking the search icon in the explorer.
- Added a "Reveal in File Explorer" / "Reveal in Finder" context menu item in the explorer. This will reveal an instance using the native OS file explorer, if that instance has a file path available.
- Added categories to the "Insert Instance" dialog.

### Changed

- The backend for the file explorer has been completely rewritten! This has been a long time coming, and all new features since version `0.2.0` have been added as a direct result of partial progress being made on this rewrite. Here's what's new:

  - Much improved general performance and responsiveness
  - More accurate git & diagnostics decorations on instances
  - Fixes several long-standing bugs (check full list below)

### Fixed

- Fixed instances sometimes getting duplicated in the explorer view.
- Fixed instances sometimes not being revealed in the explorer view when the extension starts up.
- Fixed context menu items such as "rename" and "delete" not always being available when they should be.
- Fixed the explorer view forcefully stealing focus when opening a file by other means, eg. clicking it in search or the default VSCode explorer.
- Fixed instances not being renamed or deleted correctly when they have meta files attached to them.

## `0.2.2` - November 12th, 2023

### Changed

- Roblox UI now starts up in less than 40 milliseconds when using Rojo, eliminating most of the "loading" process and letting you interact with your instances and the Explorer right away.

### Fixed

- Fixed Rojo processes being left running in the background even when VSCode and the Roblox UI extension are closed.

## `0.2.1` - November 1st, 2023

### Added

- Added tooltips with official API descriptions and links when hovering over instances in the Explorer.
- Added a 'Collapse All' button to the Explorer panel.

### Changed

- Roblox UI no longer makes any web requests! All metadata and API info is now statically bundled - this means that the VSCode extension will work just fine without an internet connection, and that opening the extension for the first time no longer requires a long load before being usable. The extension also uses less disk space because caches no longer exist.

## `0.2.0` - October 31st, 2023

The project and VSCode extension have been renamed from **Rojo Viewer** to **Roblox UI**! <br/>

This is a breaking change, so you will need to re-download the extension and update your settings if you've used Rojo Viewer before. <br/>
Going forward this will help discovery of the project and make it clear that it does not only work when using Rojo, and has more features than just an explorer view. Extensions and binaries will also be distributed on GitHub as well as marketplaces.

### Added

- Added a new icon pack `None` which will use your current VSCode icon theme instead of a custom icon pack.

### Changed

- All icon packs are now generated and bundled together with the extension, meaning they no longer need to be downloaded separately when you start using the extension. This greatly improves startup time for the extension, and removes cases where not being connected to the internet would make the extension fail to load at all.
- The `Vanilla` icon pack now uses SVGs to take advantage of higher resolution screens.

## `0.1.14` - October 20th, 2023

### Fixed

- Fixed forcing the explorer view to be focused when selecting instance files in another view (search, native file explorer, ...)
- Fixed creation of new Folder instances making a large nested tree of subdirectories instead of a single directory.

## `0.1.13` - October 13th, 2023

### Added

- Added a new setting `explorer.showDataModel` which is off by default, hiding the data model root when viewing a single workspace. This is the same behavior that Roblox Studio has.

## `0.1.12` - October 5th, 2023

### Fixed

- Fixed some minor issues with new wally integration.

## `0.1.11` - October 4th, 2023

### Added

- Added support for showing Wally packages in a nicer format, with two new settings:

  - `wally.modifyPackagesDir` - Shows Wally package directory as Package instances in the explorer
  - `wally.showPackageVersions` - Shows Wally package version on Package instances in the explorer

## `0.1.10` - October 4th, 2023

### Fixed

- Fixed prerelease versions of rojo being detected as incompatible.

## `0.1.9` - August 25th, 2023

### Added

- Added class icons to the instance insertion dialog.

### Changed

- Improved error messages and progress windows for Roblox API & icon pack downloads.
- Added a warning and timeout if spawning a process takes too long.

## `0.1.8` - July 13th, 2023

### Added

- Added support for forked versions of Rojo that have a different format in their version strings, such as `Rojo (Quenty's Version) 7.3.0`.

### Fixed

- Fixed instances in the explorer view sometimes not being revealed when their file is opened.
- Fixed instances in the explorer view not being revealed when the explorer view initially loads.
- Fixed forcing the explorer view to be focused when selecting instance files in another view. (search, native file explorer, ...)

## `0.1.7` - June 30th, 2023

### Added

- The top-level tree item (usually 'game' / the DataModel) now automatically expands unless using multi-root workspace.

## `0.1.6` - June 25th, 2023

### Fixed

- Fixed instances being revealed in the explorer just by hovering over require links.

## `0.1.5` - June 21st, 2023

### Fixed

- Fixed not being able to open instances created from multiple files - meaning scripts with `.meta.json`, wally packages with a project file pointing to `init.lua`, ...

## `0.1.4` - June 18th, 2023

### Added

- Following a file link by ctrl/cmd+clicking it or pressing F12 will now also reveal it in the explorer view.

### Changed

- Greatly improved performance of icon pack downloads.

## `0.1.3` - May 11th, 2023

### Changed

- Changed the extension to use the native VSCode filesystem APIs instead of node, which may fix some bugs.

## `0.1.2` - May 1st, 2023

### Added

- Add support for inserting services if the root of the explorer is a DataModel.
- Add support for adding arbitrary classes using `.model.json` files, improved Insert Object dialog.

### Changed

- Made right click context menu items more consistent with the normal file explorer.

### Fixed

- Fix paste action not being available in context menu.
- Fix open file / insert object ordering in context menu not being consistent with regular file explorer.

## `0.1.1` - April 28th, 2023

### Added

- Added icon packs to extension settings.
- Added support for light theme in new icon packs.
- Added "Classic" icon pack which is based on the old Roblox Studio icons.

### Changed

- Improve performance of explorer refreshes & updates.
- Decreased extension size substantially.

## `0.1.0` - April 27th, 2023

Initial Release
