use std::collections::HashMap;

/// Map macOS key codes to QEMU QKeyCode names.
pub struct KeyCodeMap {
    map: HashMap<u16, &'static str>,
}

impl KeyCodeMap {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        // Letters
        map.insert(0, "a");   map.insert(11, "b");  map.insert(8, "c");
        map.insert(2, "d");   map.insert(14, "e");  map.insert(3, "f");
        map.insert(5, "g");   map.insert(4, "h");   map.insert(34, "i");
        map.insert(38, "j");  map.insert(40, "k");  map.insert(37, "l");
        map.insert(46, "m");  map.insert(45, "n");  map.insert(31, "o");
        map.insert(35, "p");  map.insert(12, "q");  map.insert(15, "r");
        map.insert(1, "s");   map.insert(17, "t");  map.insert(32, "u");
        map.insert(9, "v");   map.insert(13, "w");  map.insert(7, "x");
        map.insert(16, "y");  map.insert(6, "z");
        // Numbers
        map.insert(29, "0");  map.insert(18, "1");  map.insert(19, "2");
        map.insert(20, "3");  map.insert(21, "4");  map.insert(23, "5");
        map.insert(22, "6");  map.insert(26, "7");  map.insert(28, "8");
        map.insert(25, "9");
        // Modifiers
        map.insert(55, "ctrl");   map.insert(56, "shift");
        map.insert(57, "caps_lock"); map.insert(58, "alt");
        map.insert(59, "ctrl");   map.insert(60, "shift_r");
        map.insert(61, "alt_r");
        // Navigation
        map.insert(36, "ret");    map.insert(48, "tab");
        map.insert(49, "spc");    map.insert(51, "backspace");
        map.insert(53, "esc");
        map.insert(123, "left");  map.insert(124, "right");
        map.insert(125, "down");  map.insert(126, "up");
        // Function keys
        map.insert(122, "f1");  map.insert(120, "f2");  map.insert(99, "f3");
        map.insert(118, "f4");  map.insert(96, "f5");   map.insert(97, "f6");
        map.insert(98, "f7");   map.insert(100, "f8");  map.insert(101, "f9");
        map.insert(109, "f10"); map.insert(103, "f11"); map.insert(111, "f12");
        // Punctuation
        map.insert(27, "minus");     map.insert(24, "equal");
        map.insert(33, "bracket_left"); map.insert(30, "bracket_right");
        map.insert(42, "backslash"); map.insert(41, "semicolon");
        map.insert(39, "apostrophe"); map.insert(43, "comma");
        map.insert(47, "dot");       map.insert(44, "slash");
        map.insert(50, "grave_accent");
        Self { map }
    }

    pub fn lookup(&self, mac_key_code: u16) -> Option<&'static str> {
        self.map.get(&mac_key_code).copied()
    }
}

impl Default for KeyCodeMap {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseMode {
    Absolute,
    Relative,
}
