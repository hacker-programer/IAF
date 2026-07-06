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
    pub fn type_text(&self, text: &str) -> Result<(), SimulateError> {
        for ch in text.chars() {
    /// Type a string as keyboard events.
    /// Soporta letras (a-z, A-Z), números (0-9), espacio, y puntuación común.
    pub fn type_text(&self, text: &str) -> Result<(), SimulateError> {
        for ch in text.chars() {
            let key = match ch {
                'a'..='z' => Key::from_str(&ch.to_string()),
                'A'..='Z' => {
                    // Para mayúsculas: presionar Shift + letra
                    let lower = ch.to_ascii_lowercase();
                    let key = Key::from_str(&lower.to_string());
                    simulate(&EventType::KeyPress(Key::ShiftLeft))?;
                    simulate(&EventType::KeyPress(key))?;
                    simulate(&EventType::KeyRelease(key))?;
                    simulate(&EventType::KeyRelease(Key::ShiftLeft))?;
                    continue;
                }
                '0'..='9' => Key::from_str(&ch.to_string()),
                ' ' => Key::Space,
                '.' => Key::Dot,
                ',' => Key::Comma,
                '-' => Key::Minus,
                '_' => {
                    simulate(&EventType::KeyPress(Key::ShiftLeft))?;
                    simulate(&EventType::KeyPress(Key::Minus))?;
                    simulate(&EventType::KeyRelease(Key::Minus))?;
                    simulate(&EventType::KeyRelease(Key::ShiftLeft))?;
                    continue;
                }
                '/' => Key::Slash,
                '\\' => Key::BackSlash,
                ':' => {
                    simulate(&EventType::KeyPress(Key::ShiftLeft))?;
                    simulate(&EventType::KeyPress(Key::SemiColon))?;
                    simulate(&EventType::KeyRelease(Key::SemiColon))?;
                    simulate(&EventType::KeyRelease(Key::ShiftLeft))?;
                    continue;
                }
                '\n' | '\r' => Key::Return,
                '\t' => Key::Tab,
                _ => continue, // Ignorar caracteres no soportados
            };
            simulate(&EventType::KeyPress(key))?;
            simulate(&EventType::KeyRelease(key))?;
        }
        Ok(())
    }

    /// Abrir una imagen (o cualquier archivo) con la aplicación predeterminada del sistema.
    pub fn open_image(&self, path: &str) -> Result<(), std::io::Error> {
        // En Windows usamos 'cmd /c start' para abrir con el programa asociado.
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(["/C", "start", "", path])
                .spawn()?;
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            // En Unix usar 'xdg-open' o 'open' (macOS).
            let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
            Command::new(opener).arg(path).spawn()?;
            Ok(())
        }
    }
}
