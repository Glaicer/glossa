# dotool

dotool reads actions from stdin and simulates keyboard/mouse input using
Linux's uinput module. It works systemwide and supports keyboard layouts.

## Install From Packages

Packages of dotool are available on:

- [Alpine](https://pkgs.alpinelinux.org/packages?name=dotool)
- [Arch (AUR)](https://aur.archlinux.org/packages?SeB=n&K=dotool)
- [Nix](https://search.nixos.org/packages?channel=unstable&type=packages&query=dotool)
- [Void](https://voidlinux.org/packages/?q=dotool)

and potentially other platforms.

## Install From Source

With `gcc` (or equivalent), `go`, `libxkbcommon-dev` and `scdoc` installed, run:

    ./build.sh && sudo ./build.sh install

And to trigger the udev rule, run:

    sudo udevadm control --reload && sudo udevadm trigger

## Usage

See the [manpage](doc/dotool.1.scd).

## Numen and Contact

dotool was written for [Numen](https://numenvoice.org), which has a
[chat on Matrix](https://matrix.to/#/#numen:matrix.org) you're welcome to join.

You can also send questions or patches by composing an email to
[~geb/numen@lists.sr.ht](https://lists.sr.ht/~geb/numen).

## Support My Work 👀

[Thank you!](https://liberapay.com/geb)

## License

GPLv3 only, see [LICENSE](./LICENSE).

Copyright (c) 2022-2026 John Gebbie
