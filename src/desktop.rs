use rdev::{simulate, Button, EventType, Key, SimulateError};
use std::collections::HashMap;
use std::process::{Command, Child};
use std::sync::{Mutex, OnceLock};

/// Mapa estático de caracteres a teclas (LUT precomputada una sola vez).
fn char_to_key_map() -> &'static HashMap<char, (Key, bool)> {
    static MAP: OnceLock<HashMap<char, (Key, bool)>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = HashMap::new();

        // Letras minúsculas (sin shift)
        let letters = [
            ('a', Key::KeyA), ('b', Key::KeyB), ('c', Key::KeyC), ('d', Key::KeyD),
            ('e', Key::KeyE), ('f', Key::KeyF), ('g', Key::KeyG), ('h', Key::KeyH),
            ('i', Key::KeyI), ('j', Key::KeyJ), ('k', Key::KeyK), ('l', Key::KeyL),
            ('m', Key::KeyM), ('n', Key::KeyN), ('o', Key::KeyO), ('p', Key::KeyP),
            ('q', Key::KeyQ), ('r', Key::KeyR), ('s', Key::KeyS), ('t', Key::KeyT),
            ('u', Key::KeyU), ('v', Key::KeyV), ('w', Key::KeyW), ('x', Key::KeyX),
            ('y', Key::KeyY), ('z', Key::KeyZ),
        ];
        for (ch, key) in letters {
            m.insert(ch, (key, false)); // minúscula: sin shift
            m.insert(ch.to_ascii_uppercase(), (key, true)); // mayúscula: con shift
        }

        // Números
        let digits = [
            ('0', Key::Num0), ('1', Key::Num1), ('2', Key::Num2), ('3', Key::Num3),
            ('4', Key::Num4), ('5', Key::Num5), ('6', Key::Num6), ('7', Key::Num7),
            ('8', Key::Num8), ('9', Key::Num9),
        ];
        for (ch, key) in digits {
            m.insert(ch, (key, false));
        }

        // Símbolos comunes (sin shift)
        m.insert(' ', (Key::Space, false));
        m.insert('.', (Key::Dot, false));
        m.insert(',', (Key::Comma, false));
        m.insert('-', (Key::Minus, false));
        m.insert('/', (Key::Slash, false));
        m.insert('\\', (Key::BackSlash, false));
        m.insert(';', (Key::SemiColon, false));
        m.insert('\'', (Key::Quote, false));
        m.insert('=', (Key::Equal, false));
        m.insert('[', (Key::LeftBracket, false));
        m.insert(']', (Key::RightBracket, false));
        m.insert('`', (Key::BackQuote, false));

        // Símbolos con shift
        m.insert('_', (Key::Minus, true));
        m.insert(':', (Key::SemiColon, true));
        m.insert('"', (Key::Quote, true));
        m.insert('+', (Key::Equal, true));
        m.insert('{', (Key::LeftBracket, true));
        m.insert('}', (Key::RightBracket, true));
        m.insert('~', (Key::BackQuote, true));
        m.insert('!', (Key::Num1, true));
        m.insert('@', (Key::Num2, true));
        m.insert('#', (Key::Num3, true));
        m.insert('$', (Key::Num4, true));
        m.insert('%', (Key::Num5, true));
        m.insert('^', (Key::Num6, true));
        m.insert('&', (Key::Num7, true));
        m.insert('*', (Key::Num8, true));
        m.insert('(', (Key::Num9, true));
        m.insert(')', (Key::Num0, true));
        m.insert('<', (Key::Comma, true));
        m.insert('>', (Key::Dot, true));
        m.insert('?', (Key::Slash, true));
        m.insert('|', (Key::BackSlash, true));

        m
    });
}

#[derive(Debug)]
pub struct DesktopController {
    pub children: Mutex<Vec<Child>>, // store Child to keep alive
}

impl DesktopController {
    pub fn new() -> Self {
        Self { children: Mutex::new(Vec::new()) }
    }

    /// Move mouse to (x, y) absolute screen coordinates.
    pub fn move_mouse(&self, x: i32, y: i32) -> Result<(), SimulateError> {
        simulate(&EventType::MouseMove { x: x as f64, y: y as f64 })
    }

    /// Click mouse button (left, right, middle).
    pub fn click(&self, button: &str) -> Result<(), SimulateError> {
        let btn = match button.to_lowercase().as_str() {
            "left" => Button::Left,
            "right" => Button::Right,
            "middle" => Button::Middle,
            _ => return Err(SimulateError),
        };
        simulate(&EventType::ButtonPress(btn))?;
        simulate(&EventType::ButtonRelease(btn))
    }

    /// Type a string as keyboard events.
    /// Usa un HashMap estático precomputado (LUT) en lugar de match verboso.
    /// Soporta letras (a-z, A-Z), números (0-9), espacio, puntuación común y símbolos con shift.
    pub fn type_text(&self, text: &str) -> Result<(), SimulateError> {
        let map = char_to_key_map();

        for ch in text.chars() {
            match ch {
                '\n' | '\r' => {
                    simulate(&EventType::KeyPress(Key::Return))?;
                    simulate(&EventType::KeyRelease(Key::Return))?;
                }
                '\t' => {
                    simulate(&EventType::KeyPress(Key::Tab))?;
                    simulate(&EventType::KeyRelease(Key::Tab))?;
                }
                _ => {
                    if let Some(&(key, needs_shift)) = map.get(&ch) {
                        if needs_shift {
                            simulate(&EventType::KeyPress(Key::ShiftLeft))?;
                            simulate(&EventType::KeyPress(key))?;
                            simulate(&EventType::KeyRelease(key))?;
                            simulate(&EventType::KeyRelease(Key::ShiftLeft))?;
                        } else {
                            simulate(&EventType::KeyPress(key))?;
                            simulate(&EventType::KeyRelease(key))?;
                        }
                    }
                    // Ignorar caracteres no soportados (no hacer nada)
                }
            }
        }
        Ok(())
    }

    /// Launch an executable and track its child process.
    pub fn launch_executable(&self, path: &str) -> Result<u32, Box<dyn std::error::Error>> {
        let child = Command::new(path).spawn()?;
        let pid = child.id();
        self.children.lock().unwrap().push(child);
        Ok(pid)
    }
}
