package main

import (
	"bufio"
	"errors"
	"fmt"
	"git.sr.ht/~geb/dotool/xkb"
	"git.sr.ht/~geb/opt"
	"github.com/bendahl/uinput"
	"math"
	"os"
	"strconv"
	"strings"
	"time"
	"unicode"
)

var Version string

func usage() {
	fmt.Println(`dotool reads actions from stdin and simulates input using uinput.

The supported actions are:
    key CHORD...
    keydown CHORD...
    keyup CHORD...
    type TEXT
    click left/middle/right
    buttondown left/middle/right
    buttonup left/middle/right
    wheel AMOUNT
    hwheel AMOUNT
    mouseto X Y  (where X and Y are percentages between 0.0 and 1.0)
    mousemove X Y  (where X and Y are amounts to move)
    keydelay MILLISECONDS
    keyhold MILLISECONDS
    typedelay MILLISECONDS
    typehold MILLISECONDS

--list-keys    Print the possible Linux keys and exit.
--list-x-keys  Print the possible XKB keys and exit.
--version      Print the version and exit.

See 'man dotool' for the documentation.`)
}

func inform(a ...any) {
	fmt.Fprintln(os.Stderr, "dotool:", fmt.Sprint(a...))
}

func warn(a ...any) {
	fmt.Fprintln(os.Stderr, "dotool: WARNING:", fmt.Sprint(a...))
}

func log(err error) {
	if err != nil {
		warn(err.Error())
	}
}

type Chord struct {
	Super, AltGr, Ctrl, Alt, Shift bool
	Key                            int

	level uint32 // just for initKeys
}

func parseChord(keymap *xkb.Keymap, chord string) (Chord, error) {
	var c Chord
	keys := strings.Split(chord, "+")

	k := keys[len(keys)-1]
	if strings.HasPrefix(k, "k:") {
		code, err := strconv.Atoi(k[2:])
		if err != nil {
			return c, errors.New("invalid keycode: " + k[2:])
		}
		c.Key = code
	} else if strings.HasPrefix(k, "x:") {
		var ok bool
		c, ok = XKeys[k[2:]]
		if !ok {
			return c, errors.New("impossible XKB key for layout: " + k[2:])
		}
	} else {
		var ok bool
		c, ok = LinuxKeys[strings.ToLower(k)]
		if !ok {
			return c, errors.New("impossible key for layout: " + k)
		}
		if len(k) == 1 && unicode.IsUpper(rune(k[0])) {
			c.Shift = true
		}
	}

	for i := 0; i < len(keys)-1; i++ {
		switch strings.ToLower(keys[i]) {
		case "super":
			c.Super = true
		case "altgr":
			c.AltGr = true
		case "ctrl", "control":
			c.Ctrl = true
		case "alt":
			c.Alt = true
		case "shift":
			c.Shift = true
		default:
			return c, errors.New("unknown modifier: " + keys[i])
		}
	}

	return c, nil
}

func (c Chord) KeyDown(kb uinput.Keyboard) {
	if c.Super {
		log(kb.KeyDown(super))
	}
	if c.AltGr {
		log(kb.KeyDown(altgr))
	}
	if c.Ctrl {
		log(kb.KeyDown(ctrl))
	}
	if c.Alt {
		log(kb.KeyDown(alt))
	}
	if c.Shift {
		log(kb.KeyDown(shift))
	}
	log(kb.KeyDown(c.Key))
}

func (c Chord) KeyUp(kb uinput.Keyboard) {
	if c.Super {
		log(kb.KeyUp(super))
	}
	if c.AltGr {
		log(kb.KeyUp(altgr))
	}
	if c.Ctrl {
		log(kb.KeyUp(ctrl))
	}
	if c.Alt {
		log(kb.KeyUp(alt))
	}
	if c.Shift {
		log(kb.KeyUp(shift))
	}
	log(kb.KeyUp(c.Key))
}

func (c *Chord) String() string {
	var sb strings.Builder
	if c.Super {
		sb.WriteString("super+")
	}
	if c.AltGr {
		sb.WriteString("altgr+")
	}
	if c.Ctrl {
		sb.WriteString("ctrl+")
	}
	if c.Alt {
		sb.WriteString("alt+")
	}
	if c.Shift {
		sb.WriteString("shift+")
	}
	sb.WriteString("k:")
	sb.WriteString(strconv.Itoa(c.Key))
	return sb.String()
}

func listKeys(keymap *xkb.Keymap, keys map[string]Chord) {
	var margin int
	for code := 1; code < 256; code++ {
		for name := range keys {
			if len(name) > margin {
				margin = len(name)
			}
		}
	}
	for code := 1; code < 256; code++ {
		for name, chord := range keys {
			if chord.Key == code {
				fmt.Printf("%-*s %s\n", margin, name, chord.String())
			}
		}
	}
}

func cutWord(s, word string) (string, bool) {
	if s == word {
		return "", true
	}
	if strings.HasPrefix(s, word+" ") || strings.HasPrefix(s, word+"\t") {
		return s[len(word)+1:], true
	}
	return "", false
}

func mainish() int {
	var keymap *xkb.Keymap
	{
		layout := os.Getenv("DOTOOL_XKB_LAYOUT")
		if layout == "" {
			layout = os.Getenv("XKB_DEFAULT_LAYOUT")
		}
		variant := os.Getenv("DOTOOL_XKB_VARIANT")
		if variant == "" {
			variant = os.Getenv("XKB_DEFAULT_VARIANT")
		}
		if variant != "" && layout == "" {
			// Otherwise xkbcommon just ignores the variant.
			inform("you need to set $DOTOOL_XKB_LAYOUT or $XKB_DEFAULT_LAYOUT if the variant is set")
			return 1
		}
		names := xkb.RuleNames{Layout: layout, Variant: variant}

		ctx := xkb.ContextNew(xkb.ContextNoFlags)
		defer ctx.Unref()
		keymap = ctx.KeymapNewFromNames(&names, xkb.KeymapCompileNoFlags)
		if keymap == nil {
			inform("failed to compile keymap")
			return 1
		}
		defer keymap.Unref()

		initKeys(keymap)
	}

	{
		o := opt.NewOptSet()
		terminate := errors.New("")

		o.FlagFunc("h", func() error {
			usage()
			return terminate
		})
		o.Alias("h", "help")

		o.FlagFunc("list-keys", func() error {
			listKeys(keymap, LinuxKeys)
			return terminate
		})

		o.FlagFunc("list-x-keys", func() error {
			listKeys(keymap, XKeys)
			return terminate
		})

		o.FlagFunc("version", func() error {
			fmt.Println(Version)
			return terminate
		})

		err := o.Parse(true, os.Args[1:])
		if err == terminate {
			return 0
		}
		if err != nil {
			inform(err.Error())
			return 1
		}
		if len(o.Args()) > 0 {
			inform("there should be no arguments, commands are read from stdin")
			return 1
		}
	}

	keyboardName := []byte(os.Getenv("DOTOOL_KEYBOARD_NAME"))
	if len(keyboardName) == 0 {
		keyboardName = []byte("dotool keyboard")
	}
	keyboard, err := uinput.CreateKeyboard("/dev/uinput", keyboardName)
	if err != nil {
		inform(err.Error())
		return 1
	}
	defer keyboard.Close()

	const precision = 10000
	touchpad, err := uinput.CreateTouchPad("/dev/uinput", []byte("dotool absolute pointer"), 0, precision, 0, precision)
	if err != nil {
		inform(err.Error())
		return 1
	}
	defer touchpad.Close()

	mouse, err := uinput.CreateMouse("/dev/uinput", []byte("dotool relative pointer"))
	if err != nil {
		inform(err.Error())
		return 1
	}
	defer mouse.Close()

	keydelay := time.Duration(2) * time.Millisecond
	keyhold := time.Duration(8) * time.Millisecond
	typedelay := time.Duration(2) * time.Millisecond
	typehold := time.Duration(8) * time.Millisecond
	const clickdelay = time.Duration(30) * time.Millisecond

	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		text := strings.TrimLeftFunc(sc.Text(), unicode.IsSpace)
		if text == "" {
			continue
		}
		if s, ok := cutWord(text, "key"); ok {
			for _, field := range strings.Fields(s) {
				if chord, err := parseChord(keymap, field); err == nil {
					chord.KeyDown(keyboard)
					time.Sleep(keyhold)
					chord.KeyUp(keyboard)
				} else {
					warn(err.Error())
				}
				time.Sleep(keydelay)
			}
		} else if s, ok := cutWord(text, "keydown"); ok {
			for _, field := range strings.Fields(s) {
				if chord, err := parseChord(keymap, field); err == nil {
					chord.KeyDown(keyboard)
				} else {
					warn(err.Error())
				}
				time.Sleep(keydelay)
			}
		} else if s, ok := cutWord(text, "keyup"); ok {
			for _, field := range strings.Fields(s) {
				if chord, err := parseChord(keymap, field); err == nil {
					chord.KeyUp(keyboard)
				} else {
					warn(err.Error())
				}
				time.Sleep(keydelay)
			}
		} else if s, ok := cutWord(text, "keydelay"); ok {
			var d float64
			_, err := fmt.Sscanf(s+"\n", "%f\n", &d)
			if err == nil {
				keydelay = time.Duration(d) * time.Millisecond
			} else {
				warn("invalid delay: " + sc.Text())
			}
		} else if s, ok := cutWord(text, "keyhold"); ok {
			var d float64
			_, err := fmt.Sscanf(s+"\n", "%f\n", &d)
			if err == nil {
				keyhold = time.Duration(d) * time.Millisecond
			} else {
				warn("invalid hold time: " + sc.Text())
			}
		} else if s, ok := cutWord(text, "type"); ok {
			for _, r := range s {
				sym := xkb.Utf32ToKeysym(uint32(r))
				if sym == 0 {
					warn("invalid character: " + string(r))
				} else if c := getChord(keymap, sym); c.Key != 0 {
					c.KeyDown(keyboard)
					time.Sleep(typehold)
					c.KeyUp(keyboard)
				} else if c1, c2 := getDeadChords(keymap, r); c1.Key != 0 {
					c1.KeyDown(keyboard)
					time.Sleep(typehold)
					c1.KeyUp(keyboard)
					time.Sleep(typedelay)
					c2.KeyDown(keyboard)
					time.Sleep(typehold)
					c2.KeyUp(keyboard)
				} else {
					warn("impossible character for layout: " + string(r))
				}
				time.Sleep(typedelay)
			}
		} else if s, ok := cutWord(text, "typedelay"); ok {
			var d float64
			_, err := fmt.Sscanf(s+"\n", "%f\n", &d)
			if err == nil {
				typedelay = time.Duration(d) * time.Millisecond
			} else {
				warn("invalid delay: " + sc.Text())
			}
		} else if s, ok := cutWord(text, "typehold"); ok {
			var d float64
			_, err := fmt.Sscanf(s+"\n", "%f\n", &d)
			if err == nil {
				typehold = time.Duration(d) * time.Millisecond
			} else {
				warn("invalid hold time: " + sc.Text())
			}
		} else if s, ok := cutWord(text, "click"); ok {
			for _, button := range strings.Fields(s) {
				switch button {
				case "left", "1":
					log(mouse.LeftClick())
				case "middle", "2":
					log(mouse.MiddleClick())
				case "right", "3":
					log(mouse.RightClick())
				default:
					warn("unknown button: " + button)
				}
				time.Sleep(clickdelay)
			}
		} else if s, ok := cutWord(text, "buttondown"); ok {
			for _, button := range strings.Fields(s) {
				switch button {
				case "left", "1":
					log(mouse.LeftPress())
				case "middle", "2":
					log(mouse.MiddlePress())
				case "right", "3":
					log(mouse.RightPress())
				default:
					warn("unknown button: " + button)
				}
			}
		} else if s, ok := cutWord(text, "buttonup"); ok {
			for _, button := range strings.Fields(s) {
				switch button {
				case "left", "1":
					log(mouse.LeftRelease())
				case "middle", "2":
					log(mouse.MiddleRelease())
				case "right", "3":
					log(mouse.RightRelease())
				default:
					warn("unknown button: " + button)
				}
			}
		} else if s, ok := cutWord(text, "wheel"); ok {
			var n float64
			_, err := fmt.Sscanf(s+"\n", "%f\n", &n)
			if err == nil {
				log(mouse.Wheel(false, int32(n)))
			} else {
				warn("invalid wheel amount: " + sc.Text())
			}
		} else if s, ok := cutWord(text, "hwheel"); ok {
			var n float64
			_, err := fmt.Sscanf(s+"\n", "%f\n", &n)
			if err == nil {
				log(mouse.Wheel(true, int32(n)))
			} else {
				warn("invalid hwheel amount: " + sc.Text())
			}
		} else if s, ok := cutWord(text, "scroll"); ok {
			var n float64
			_, err := fmt.Sscanf(s+"\n", "%f\n", &n)
			if err == nil {
				log(mouse.Wheel(false, int32(-n)))
			} else {
				warn("invalid scroll amount: " + sc.Text())
			}
		} else if s, ok := cutWord(text, "mouseto"); ok {
			var x, y float64
			_, err := fmt.Sscanf(s+"\n", "%f %f\n", &x, &y)
			if err == nil {
				if x < 0.0 || x > 1.0 || y < 0.0 || y > 1.0 {
					warn("clamping percentages between 0.0 and 1.0: " + sc.Text())
					x = math.Max(0, math.Min(x, 1.0))
					y = math.Min(0, math.Min(y, 1.0))
				}
				x, y := int32(x*float64(precision)), int32(y*float64(precision))
				err := touchpad.MoveTo(x, y)
				if err != nil {
					warn(err.Error())
				}
			} else {
				warn("invalid coordinate: " + sc.Text())
			}
		} else if s, ok := cutWord(text, "mousemove"); ok {
			var x, y float64
			_, err := fmt.Sscanf(s+"\n", "%f %f\n", &x, &y)
			if err == nil {
				err := mouse.Move(int32(x), int32(y))
				if err != nil {
					warn(err.Error())
				}
			} else {
				warn("invalid coordinate: " + sc.Text())
			}
		} else {
			warn("unknown command: " + sc.Text())
		}
	}
	if sc.Err() != nil {
		inform(sc.Err().Error())
		return 1
	}
	return 0
}

func main() {
	os.Exit(mainish())
}
