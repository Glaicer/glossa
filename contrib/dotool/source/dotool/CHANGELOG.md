# Changelog

Notable changes to dotool will be documented in this file.

## [1.6](https://git.sr.ht/~geb/dotool/refs/1.6)

### Changed

- click now has a minuscule delay so multi-clicks are more robust.
- uinput udev rule MODE 0660 -> 0620, because only write is necessary.

## [1.5](https://git.sr.ht/~geb/dotool/refs/1.5)

### Fixed

- The selecting of the fewest modifiers for simulating keys.

## [1.4](https://git.sr.ht/~geb/dotool/refs/1.4)

### Added

- A manpage, requiring scdoc.
- Heuristic support for dead keys.
- Support for altgr.
- $DOTOOL_KEYBOARD_NAME to set the virtual keyboard's name.
- More verbose --list-keys output.

### Changed

- Replaced ./install.sh with ./build.sh.

### Fixed

- Now prefers the fewest modifiers for simulating keys. (UPDATE: see 1.5)

## [1.3](https://git.sr.ht/~geb/dotool/refs/1.3)

### Added

- Support for keyboard layouts.
- hwheel for horizontal scrolling.

### Changed

- Now depends on the xkbcommon library.
- XKB key names are now case-sensitive.
- scroll -> wheel

## [1.2](https://git.sr.ht/~geb/dotool/refs/1.2)

### Added

- Added X11 and sway config files for the virtual keyboard's layout.
- Added ./\_install.sh for packaging the basic files.

### Fixed

- Stopped some x: keys simulating shift.

## [1.1](https://git.sr.ht/~geb/dotool/refs/1.1)

### Added

- Added --version.
- Added keyhold and typehold.

### Changed

- There is now a default keydelay, keyhold, typedelay and typehold.
- Delays now come after keypresses not before.
- Now aborts if there are any command-line arguments.

### Fixed

- Added a udev rule so it shouldn't type guff if you're using a non-us
keyboard layout on at least X11.
- Fixed x:backspace that was simulating shift.
