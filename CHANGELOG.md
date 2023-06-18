<!-- markdownlint-disable MD023 -->
<!-- markdownlint-disable MD033 -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Changed

-   Greatly improved performance of icon pack downloads

## `0.1.3` - May 11th, 2023

### Changed

-   Changed the extension to use the native VSCode filesystem APIs instead of node, which may fix some bugs

## `0.1.2` - May 1st, 2023

### Added

-   Add support for inserting services if the root of the explorer is a DataModel
-   Add support for adding arbitrary classes using `.model.json` files, improved Insert Object dialog

### Changed

-   Made right click context menu items more consistent with the normal file explorer

### Fixed

-   Fix paste action not being available in context menu
-   Fix open file / insert object ordering in context menu not being consistent with regular file explorer

## `0.1.1` - April 28th, 2023

### Added

-   Added icon packs to extension settings
-   Added support for light theme in new icon packs
-   Added "Classic" icon pack which is based on the old Roblox Studio icons

### Changed

-   Improve performance of explorer refreshes & updates
-   Decreased extension size substantially

## `0.1.0` - April 27th, 2023

Initial Release
