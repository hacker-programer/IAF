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
            match ch {
                ' ' => {
                    simulate(&EventType::KeyPress(Key::Space))?;
                    simulate(&EventType::KeyRelease(Key::Space))?;
                }
                _ => continue, // ignore unsupported characters for now
            }
        }
        Ok(())
    }

    /// Launch an executable file and keep the child handle.
    pub fn launch_executable(&self, path: &str) -> Result<u32, std::io::Error> {
        let child = Command::new(path).spawn()?;
        let pid = child.id();
        self.children.lock().unwrap().push(child);
        Ok(pid)
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
