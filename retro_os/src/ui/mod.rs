//! RetroOS UI Module - Retro Mac OS Style Interface
//! 
//! Priority #2: Lightweight UI inspired by classic Mac OS (System 7/OS 9)
//! Text-based and simple graphical interface for low-resource systems

use anyhow::Result;
use log::{info, debug};
use std::io::{self, Write, Stdout};
use crossterm::{
    execute, terminal, cursor, style,
    event::{self, Event, KeyCode, KeyEvent},
};
use style::Color;

/// Color palette inspired by classic Mac OS
pub mod colors {
    use crossterm::style::Color;
    
    pub const MENU_BAR_BG: Color = Color::White;
    pub const MENU_BAR_FG: Color = Color::Black;
    pub const WINDOW_BG: Color = Color::White;
    pub const WINDOW_FG: Color = Color::Black;
    pub const HIGHLIGHT: Color = Color::Blue;
    pub const BUTTON_BG: Color = Color::White;
    pub const BUTTON_FG: Color = Color::Black;
    pub const DESKTOP_BG: Color = Color::DarkGrey;
}

/// Window representation
#[derive(Debug, Clone)]
pub struct Window {
    pub id: usize,
    pub title: String,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub is_active: bool,
    pub content: Vec<String>,
}

impl Window {
    pub fn new(id: usize, title: &str, x: u16, y: u16, width: u16, height: u16) -> Self {
        Window {
            id,
            title: title.to_string(),
            x,
            y,
            width,
            height,
            is_active: false,
            content: Vec::new(),
        }
    }
}

/// Menu bar with classic Mac OS style
pub struct MenuBar {
    items: Vec<String>,
}

impl MenuBar {
    pub fn new() -> Self {
        MenuBar {
            items: vec![
                "🍎".to_string(), // Apple menu
                "File".to_string(),
                "Edit".to_string(),
                "View".to_string(),
                "Help".to_string(),
            ],
        }
    }

    pub fn render(&self, stdout: &mut Stdout, width: u16) -> Result<()> {
        execute!(
            stdout,
            cursor::MoveTo(0, 0),
            style::SetBackgroundColor(colors::MENU_BAR_BG),
            style::SetForegroundColor(colors::MENU_BAR_FG),
        )?;

        // Draw menu bar background
        for x in 0..width {
            execute!(stdout, cursor::MoveTo(x, 0), style::Print(" "))?;
        }

        // Draw menu items
        let mut x_pos = 1;
        execute!(stdout, cursor::MoveTo(x_pos, 0))?;
        for item in &self.items {
            execute!(stdout, style::Print(item.clone()), style::Print("  "))?;
            x_pos += item.len() as u16 + 2;
        }

        Ok(())
    }
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Desktop environment
pub struct Desktop {
    windows: Vec<Window>,
    active_window_id: Option<usize>,
    next_window_id: usize,
    menu_bar: MenuBar,
}

impl Desktop {
    pub fn new() -> Self {
        Desktop {
            windows: Vec::new(),
            active_window_id: None,
            next_window_id: 1,
            menu_bar: MenuBar::new(),
        }
    }

    /// Create a new window
    pub fn create_window(&mut self, title: &str, x: u16, y: u16, width: u16, height: u16) -> usize {
        let id = self.next_window_id;
        self.next_window_id += 1;
        
        let mut window = Window::new(id, title, x, y, width, height);
        window.is_active = true;
        
        // Deactivate other windows
        for w in &mut self.windows {
            w.is_active = false;
        }
        
        self.windows.push(window);
        self.active_window_id = Some(id);
        
        info!("Created window '{}' with ID {}", title, id);
        id
    }

    /// Close a window
    pub fn close_window(&mut self, id: usize) -> bool {
        if let Some(pos) = self.windows.iter().position(|w| w.id == id) {
            self.windows.remove(pos);
            
            // Set another window as active if available
            if let Some(last) = self.windows.last() {
                self.active_window_id = Some(last.id);
            } else {
                self.active_window_id = None;
            }
            
            info!("Closed window with ID {}", id);
            true
        } else {
            false
        }
    }

    /// Add content to a window
    pub fn add_content(&mut self, id: usize, line: &str) -> bool {
        if let Some(window) = self.windows.iter_mut().find(|w| w.id == id) {
            window.content.push(line.to_string());
            true
        } else {
            false
        }
    }

    /// Render the desktop
    pub fn render(&self, stdout: &mut Stdout) -> Result<()> {
        let (width, height) = terminal::size()?;
        
        // Clear screen with desktop background color
        execute!(
            stdout,
            style::SetBackgroundColor(colors::DESKTOP_BG),
            terminal::Clear(terminal::ClearType::All),
        )?;

        // Render menu bar
        self.menu_bar.render(stdout, width)?;

        // Render windows
        for window in &self.windows {
            self.render_window(stdout, window)?;
        }

        stdout.flush()?;
        Ok(())
    }

    /// Render a single window
    fn render_window(&self, stdout: &mut Stdout, window: &Window) -> Result<()> {
        let border_color = if window.is_active {
            style::Color::Black
        } else {
            style::Color::DarkGrey
        };

        // Draw window border (Mac OS style with double lines simulation)
        for y in window.y..window.y + window.height {
            for x in window.x..window.x + window.width {
                let ch = if y == window.y || y == window.y + window.height - 1 {
                    '=' // Horizontal border
                } else if x == window.x || x == window.x + window.width - 1 {
                    '|' // Vertical border
                } else {
                    ' ' // Interior
                };

                execute!(
                    stdout,
                    cursor::MoveTo(x, y),
                    style::SetForegroundColor(border_color),
                    style::Print(ch),
                )?;
            }
        }

        // Draw title bar
        if window.y < window.y + window.height {
            execute!(
                stdout,
                cursor::MoveTo(window.x + 2, window.y),
                style::SetForegroundColor(style::Color::Black),
                style::Print(format!(" {} ", window.title)),
            )?;
        }

        // Draw window content
        for (i, line) in window.content.iter().enumerate() {
            let content_y = window.y + 2 + i as u16;
            if content_y < window.y + window.height - 1 {
                execute!(
                    stdout,
                    cursor::MoveTo(window.x + 1, content_y),
                    style::SetForegroundColor(style::Color::Black),
                    style::Print(&line[..(window.width as usize - 2).min(line.len())]),
                )?;
            }
        }

        Ok(())
    }

    /// Get the number of open windows
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }
}

impl Default for Desktop {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize terminal for UI
pub fn init_terminal() -> Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        cursor::Hide,
    )?;
    Ok(())
}

/// Restore terminal to normal state
pub fn restore_terminal() -> Result<()> {
    terminal::disable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        terminal::LeaveAlternateScreen,
        cursor::Show,
    )?;
    Ok(())
}

/// Run a simple text-based demo
pub fn run_demo() -> Result<()> {
    info!("Starting RetroOS UI demo");
    
    init_terminal()?;
    
    let mut desktop = Desktop::new();
    let mut stdout = io::stdout();
    
    // Create sample windows
    let terminal_id = desktop.create_window("Terminal", 2, 3, 60, 15);
    desktop.add_content(terminal_id, "Welcome to RetroOS v0.1.0");
    desktop.add_content(terminal_id, "Running on Linux kernel compatibility layer");
    desktop.add_content(terminal_id, "$ _");
    
    let finder_id = desktop.create_window("Finder", 10, 5, 50, 12);
    desktop.add_content(finder_id, "📁 Applications");
    desktop.add_content(finder_id, "📁 Documents");
    desktop.add_content(finder_id, "📁 System");
    
    desktop.render(&mut stdout)?;
    
    // Wait for key press
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('n') => {
                        // Create new window
                        let id = desktop.create_window(
                            "New Window",
                            15, 8, 40, 10,
                        );
                        desktop.add_content(id, "This is a new window");
                        desktop.render(&mut stdout)?;
                    }
                    KeyCode::Char('c') => {
                        // Close active window
                        if let Some(active_id) = desktop.active_window_id {
                            desktop.close_window(active_id);
                            desktop.render(&mut stdout)?;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    restore_terminal()?;
    info!("UI demo completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_creation() {
        let desktop = Desktop::new();
        assert_eq!(desktop.window_count(), 0);
    }

    #[test]
    fn test_window_management() {
        let mut desktop = Desktop::new();
        
        let id = desktop.create_window("Test", 0, 0, 10, 10);
        assert_eq!(desktop.window_count(), 1);
        assert_eq!(desktop.active_window_id, Some(id));
        
        desktop.close_window(id);
        assert_eq!(desktop.window_count(), 0);
    }
}
