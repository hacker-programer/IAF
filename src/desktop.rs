use rdev::{simulate, Button, EventType, Key, SimulateError};
use std::process::{Command, Child};
use std::sync::Mutex;

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

    /// Type a string as keyboard events (simplified).
    /// Type a string as keyboard events.
    /// Soporta letras (a-z, A-Z), números (0-9), espacio, y puntuación común.
    pub fn type_text(&self, text: &str) -> Result<(), SimulateError> {
        for ch in text.chars() {
            match ch {
                'a'..='z' => {
                        'a' => Key::KeyA, 'b' => Key::KeyB, 'c' => Key::KeyC,
                        'd' => Key::KeyD, 'e' => Key::KeyE, 'f' => Key::KeyF,
                        'g' => Key::KeyG, 'h' => Key::KeyH, 'i' => Key::KeyI,
                        'j' => Key::KeyJ, 'k' => Key::KeyK, 'l' => Key::KeyL,
                        'm' => Key::KeyM, 'n' => Key::KeyN, 'o' => Key::KeyO,
                        'p' => Key::KeyP, 'q' => Key::KeyQ, 'r' => Key::KeyR,
                        's' => Key::KeyS, 't' => Key::KeyT, 'u' => Key::KeyU,
                        'v' => Key::KeyV, 'w' => Key::KeyW, 'x' => Key::KeyX,
                        'y' => Key::KeyY, 'z' => Key::KeyZ,
                        _ => continue,
                    };
                    simulate(&EventType::KeyPress(key))?;
                    simulate(&EventType::KeyRelease(key))?;
                }
                'A'..='Z' => {
                    let lower = ch.to_ascii_lowercase();
                    let key = match lower {
                        'a' => Key::KeyA, 'b' => Key::KeyB, 'c' => Key::KeyC,
                        'd' => Key::KeyD, 'e' => Key::KeyE, 'f' => Key::KeyF,
                        'g' => Key::KeyG, 'h' => Key::KeyH, 'i' => Key::KeyI,
                        'j' => Key::KeyJ, 'k' => Key::KeyK, 'l' => Key::KeyL,
                        'm' => Key::KeyM, 'n' => Key::KeyN, 'o' => Key::KeyO,
                        'p' => Key::KeyP, 'q' => Key::KeyQ, 'r' => Key::KeyR,
                        's' => Key::KeyS, 't' => Key::KeyT, 'u' => Key::KeyU,
                        'v' => Key::KeyV, 'w' => Key::KeyW, 'x' => Key::KeyX,
                        'y' => Key::KeyY, 'z' => Key::KeyZ,
                        _ => continue,
                    };
                    simulate(&EventType::KeyPress(Key::ShiftLeft))?;
                    simulate(&EventType::KeyPress(key))?;
                    simulate(&EventType::KeyRelease(key))?;
                    simulate(&EventType::KeyRelease(Key::ShiftLeft))?;
                }
                '0'..='9' => {
                    let key = match ch {
                        '0' => Key::Num0, '1' => Key::Num1, '2' => Key::Num2,
                        '3' => Key::Num3, '4' => Key::Num4, '5' => Key::Num5,
                        '6' => Key::Num6, '7' => Key::Num7, '8' => Key::Num8,
                        '9' => Key::Num9,
                        _ => continue,
                    };
                    simulate(&EventType::KeyPress(key))?;
                    simulate(&EventType::KeyRelease(key))?;
                }
                ' ' => {
                    simulate(&EventType::KeyPress(Key::Space))?;
                    simulate(&EventType::KeyRelease(Key::Space))?;
                }
                '.' => {
                    simulate(&EventType::KeyPress(Key::Dot))?;
                    simulate(&EventType::KeyRelease(Key::Dot))?;
                }
                ',' => {
                    simulate(&EventType::KeyPress(Key::Comma))?;
                    simulate(&EventType::KeyRelease(Key::Comma))?;
                }
                '-' => {
                    simulate(&EventType::KeyPress(Key::Minus))?;
                    simulate(&EventType::KeyRelease(Key::Minus))?;
                }
                '_' => {
                    simulate(&EventType::KeyPress(Key::ShiftLeft))?;
                    simulate(&EventType::KeyPress(Key::Minus))?;
                    simulate(&EventType::KeyRelease(Key::Minus))?;
                    simulate(&EventType::KeyRelease(Key::ShiftLeft))?;
                }
                '/' => {
                    simulate(&EventType::KeyPress(Key::Slash))?;
                    simulate(&EventType::KeyRelease(Key::Slash))?;
                }
                '\\' => {
                    simulate(&EventType::KeyPress(Key::BackSlash))?;
                    simulate(&EventType::KeyRelease(Key::BackSlash))?;
                }
                ':' => {
                    simulate(&EventType::KeyPress(Key::ShiftLeft))?;
                    simulate(&EventType::KeyPress(Key::SemiColon))?;
                    simulate(&EventType::KeyRelease(Key::SemiColon))?;
                    simulate(&EventType::KeyRelease(Key::ShiftLeft))?;
                }
                '\n' | '\r' => {
                    simulate(&EventType::KeyPress(Key::Return))?;
                    simulate(&EventType::KeyRelease(Key::Return))?;
                }
                '\t' => {
                    simulate(&EventType::KeyPress(Key::Tab))?;
                    simulate(&EventType::KeyRelease(Key::Tab))?;
                }
                _ => continue, // Ignorar caracteres no soportados
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