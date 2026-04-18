package xkb

//#cgo pkg-config: xkbcommon
//#cgo LDFLAGS: -ldl
//
//#include <stdlib.h>
//#include <xkbcommon/xkbcommon.h>
import "C"
import "unsafe"

const (
	// ModNameShift is the name of Shift Modifier
	ModNameShift = "Shift"
	// ModNameCaps is the name of Caps Lock Modifier
	ModNameCaps = "Lock"
	// ModNameCtrl is the name of Control Modifier
	ModNameCtrl = "Control"
	// ModNameAlt is the name of Alt Modifier
	ModNameAlt = "Mod1"
	// ModNameNum is the name of Num Lock Modifier
	ModNameNum = "Mod2"
	// ModNameLogo is the name of Logo Modifier
	ModNameLogo = "Mod4"
	// LedNameCaps is the name of Caps Lock Led
	LedNameCaps = "Caps Lock"
	// LedNameNum is the name of Num Lock Led
	LedNameNum = "Num Lock"
	// LedNameScroll is the name of Scroll Lock Led
	LedNameScroll = "Scroll Lock"
)

func KeysymGetName(keysym uint32) string {
	s := "................................................................"
	cs := C.CString(s)
	defer C.free(unsafe.Pointer(cs))
	_ = C.xkb_keysym_get_name(C.uint(keysym), cs, C.size_t(len(s)))
	return C.GoString(cs)
}

func KeysymToUtf32(keysym uint32) uint32 {
	return uint32(C.xkb_keysym_to_utf32(C.uint(keysym)))
}

func Utf32ToKeysym(ucs uint32) uint32 {
	return uint32(C.xkb_utf32_to_keysym(C.uint(ucs)))
}

type KeymapCompileFlags int

const (
	KeymapCompileNoFlags KeymapCompileFlags = C.XKB_KEYMAP_COMPILE_NO_FLAGS
)

type RuleNames struct {
	Rules, Model, Layout, Variant, Options string
}

func (rn *RuleNames) toC() *C.struct_xkb_rule_names {
	if rn == nil {
		return nil
	}
	return &C.struct_xkb_rule_names{
		rules:   C.CString(rn.Rules),
		model:   C.CString(rn.Model),
		layout:  C.CString(rn.Layout),
		variant: C.CString(rn.Variant),
		options: C.CString(rn.Options),
	}
}

const ContextNoFlags = 0

type Context struct {
	p *C.struct_xkb_context
}

func ContextNew(flags uint32) (ctx *Context) {
	ctx = new(Context)
	ctx.p = C.xkb_context_new(flags)
	return ctx
}

func (ctx *Context) KeymapNewFromNames(rules *RuleNames, flags KeymapCompileFlags) *Keymap {
	km := C.xkb_keymap_new_from_names(ctx.p, rules.toC(), C.enum_xkb_keymap_compile_flags(flags))
	if km == nil {
		return nil
	}
	return &Keymap{km}
}

func (ctx *Context) Unref() {
	C.xkb_context_unref(ctx.p)
	ctx.p = nil
}

type Keymap struct {
	p *C.struct_xkb_keymap
}

func (km *Keymap) KeyGetMod(key, layout, level uint32) uint32 {
	var mask C.uint
	C.xkb_keymap_key_get_mods_for_level(km.p, C.uint(key), C.uint(layout), C.uint(level), &mask, 1)
	return uint32(mask)
}

func (km *Keymap) KeyGetSymsByLevel(key, layout, level uint32) []uint32 {
	var syms *C.xkb_keysym_t
	n := int(C.xkb_keymap_key_get_syms_by_level(km.p, C.uint(key), C.uint(layout), C.uint(level), &syms))
	if n == 0 || syms == nil {
		return nil
	}
	data := unsafe.Slice(syms, n)
	s := make([]uint32, n)
	for i := 0; i < n; i++ {
		s[i] = uint32(data[i])
	}
	return s
}

func (km *Keymap) MaxKeycode() uint32 {
	return uint32(C.xkb_keymap_max_keycode(km.p))
}

func (km *Keymap) MinKeycode() uint32 {
	return uint32(C.xkb_keymap_min_keycode(km.p))
}

func (km *Keymap) ModGetIndex(mod string) uint {
	return uint(C.xkb_keymap_mod_get_index(km.p, C.CString(mod)))
}

func (km *Keymap) NumLevelsForKey(key, layout uint32) uint32 {
	return uint32(C.xkb_keymap_num_levels_for_key(km.p, C.uint(key), C.uint(layout)))
}

func (km *Keymap) Unref() {
	C.xkb_keymap_unref(km.p)
	km.p = nil
}

func keymapUnref(km *Keymap) {
	C.xkb_keymap_unref(km.p)
	km.p = nil
}
