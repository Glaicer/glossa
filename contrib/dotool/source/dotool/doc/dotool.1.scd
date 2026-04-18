dotool(1)

# NAME

*dotool* - uinput tool

# SYNOPSIS

*dotool* < _actions_

# DESCRIPTION

*dotool* reads actions from stdin and simulates keyboard/mouse input using
Linux's uinput module.

# PERMISSION

*dotool* requires write permission to */dev/uinput*, which is granted to
users in group *input* by a udev rule.

You can test:

	*echo type hello | dotool*

and if need be, you could add your user to group *input* with:

	*groupadd -f input*++
*usermod -a -G input $USER*

and then it's foolproof to reboot to make the group and rule effective.

# KEYBOARD LAYOUTS

*dotool* may type gobbledygook if it's simulating keycodes for a different
keyboard layout than your environment is expecting.

You can specify the layout with the environment variables *DOTOOL_XKB_LAYOUT*
and *DOTOOL_XKB_VARIANT*. For example:

	*echo type azerty | DOTOOL_XKB_LAYOUT=fr dotool*

You can also specify the name to give the virtual keyboard with the environment
variable *DOTOOL_KEYBOARD_NAME*, which can be useful making rules for your
environment.

Currently the *type* action has only heuristic support for dead keys.

# OPTIONS

*-h*, *--help*
	Print help and exit.

*--list-keys*
	Print the possible Linux keys and exit.

*--list-x-keys*
	Print the possible XKB keys and exit.

*--version*
	Print the version and exit.

# ACTIONS

*key* _CHORD_...++
*keydown* _CHORD_...++
*keyup* _CHORD_...
	Press and/or release each _CHORD_. A _CHORD_ is a key or a key with
	modifiers, such as *a*, *shift+a* or *ctrl+shift+a*.

	The supported modifiers are *super*, *altgr*, *ctrl*, *alt* and *shift*.

	Keys can be specified by Linux names, XKB names prefixed with
	*x:*, or Linux keycodes prefixed with *k:*. The Linux names are
	case-insensitive, except uppercase character keys also simulate
	shift. This example types *!!!* with the *us* layout:

		*echo key shift+1 x:exclam shift+k:2 | dotool*

*type* _TEXT_
	Type _TEXT_.

*click* *left*/*middle*/*right*++
*buttondown* *left*/*middle*/*right*++
*buttonup* *left*/*middle*/*right*
	Press and/or release a mouse button.

*wheel* _AMOUNT_++
*hwheel* _AMOUNT_
	Scroll a vertical/horizontal mouse wheel by a positive or negative
	_AMOUNT_.

*mouseto* _X_ _Y_
	Jump the cursor to the position _X_ _Y_, where _X_ and _Y_ are
	percentages between 0.0 and 1.0.

*mousemove* _X_ _Y_
	Move the cursor relative to its current position.

*keydelay* _MILLISECONDS_++
*keyhold* _MILLISECONDS_++
*typedelay* _MILLISECONDS_++
*typehold* _MILLISECONDS_
	Set the delay between/holding each key with the *key*\* actions/*type*
	action.

	The default *keydelay* and *typedelay* is 2ms, and the default
	*keyhold* and *typehold* is 8ms.

# LONG-RUNNING INSTANCE

Each instance of *dotool* has an initial delay registering the virtual
devices, but you can keep writing actions to a long-running instance. The
daemon and client, *dotoold* and *dotoolc*, let you do this with a pipe
behind the scenes, for example:

	*dotoold &*++
*echo type super | dotoolc*++
*echo type speedy | dotoolc*

# EXAMPLES

This greets the world:

	*echo type hi | dotool*

This screams for roughly three seconds:

	*{ echo keydown A; sleep 3; echo key H shift+1; } | dotool*

# AUTHOR

John Gebbie
