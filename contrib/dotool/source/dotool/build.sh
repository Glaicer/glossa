#!/bin/sh
# ./build.sh ['install']
: "${DOTOOL_VERSION=$(git describe --long --abbrev=12 --tags --dirty 2>/dev/null || echo 1.6)}"
: "${DOTOOL_DESTDIR=}"
: "${DOTOOL_BINDIR=usr/local/bin}"
: "${DOTOOL_UDEV_RULES_DIR=etc/udev/rules.d}"

if [ "$*" != '' ] && [ "$*" != install ]; then
	echo bad usage
	exit 1
fi

if ! [ "$1" ]; then
	go build -ldflags "-X main.Version=$DOTOOL_VERSION" || exit
	echo Built Successfully.
else
	install -Dm755 dotool dotoolc dotoold -t "$DOTOOL_DESTDIR/$DOTOOL_BINDIR" || exit
	install -Dm644 80-dotool.rules -t "$DOTOOL_DESTDIR/$DOTOOL_UDEV_RULES_DIR" || exit
	mkdir -p "$DOTOOL_DESTDIR/usr/share/man/man1" || exit
	scdoc < doc/dotool.1.scd > "$DOTOOL_DESTDIR/usr/share/man/man1/dotool.1" || exit
	echo Installed Successfully.
fi
