use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Operating system platform for platform-specific keyboard maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Windows,
    Linux,
    MacOS,
}

impl Platform {
    /// Detects current host OS platform.
    pub fn current() -> Self {
        #[cfg(target_os = "macos")]
        {
            Platform::MacOS
        }
        #[cfg(target_os = "windows")]
        {
            Platform::Windows
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Platform::Linux
        }
    }

    /// Whether this platform uses Command instead of Ctrl as the primary accelerator modifier.
    pub fn uses_command_key(&self) -> bool {
        matches!(self, Platform::MacOS)
    }
}

/// Navigation direction for cursor and selection movements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Semantic actions triggered by keyboard shortcuts in the spreadsheet application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShortcutAction {
    // File operations
    NewWorkbook,
    OpenWorkbook,
    SaveWorkbook,
    SaveAsWorkbook,

    // Edit operations & Clipboard
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,

    // Navigation
    MoveCursor(Direction),
    ExtendSelection(Direction),
    JumpCursor(Direction),
    ExtendJumpSelection(Direction),
    SelectAll,
    SelectRow,
    SelectColumn,
    GoToHome,
    GoToEnd,
    PageUp,
    PageDown,

    // Cell editing
    EditCell,
    Cancel,
    CommitAndMoveDown,
    CommitAndMoveUp,
    CommitAndMoveRight,
    CommitAndMoveLeft,
    ToggleAbsoluteReference, // F4
    AutoSum,                 // Alt+=
    ClearContents,           // Delete
    Backspace,

    // Formatting & View
    FormatCells, // Ctrl+1
    Bold,        // Ctrl+B
    Italic,      // Ctrl+I
    Underline,   // Ctrl+U
    Find,        // Ctrl+F
    Replace,     // Ctrl+H
    Recalculate, // F9
}

/// Abstract representation of keyboard modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub mac_cmd: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        ctrl: false,
        alt: false,
        shift: false,
        mac_cmd: false,
    };
    pub const CTRL: Self = Self {
        ctrl: true,
        alt: false,
        shift: false,
        mac_cmd: false,
    };
    pub const ALT: Self = Self {
        ctrl: false,
        alt: true,
        shift: false,
        mac_cmd: false,
    };
    pub const SHIFT: Self = Self {
        ctrl: false,
        alt: false,
        shift: true,
        mac_cmd: false,
    };
    pub const CMD: Self = Self {
        ctrl: false,
        alt: false,
        shift: false,
        mac_cmd: true,
    };
    pub const CTRL_SHIFT: Self = Self {
        ctrl: true,
        alt: false,
        shift: true,
        mac_cmd: false,
    };
    pub const CMD_SHIFT: Self = Self {
        ctrl: false,
        alt: false,
        shift: true,
        mac_cmd: true,
    };
    pub const ALT_SHIFT: Self = Self {
        ctrl: false,
        alt: true,
        shift: true,
        mac_cmd: false,
    };

    /// Matches against egui's native modifiers state considering platform conventions.
    pub fn matches_egui(&self, egui_mods: &egui::Modifiers, platform: Platform) -> bool {
        let expected_cmd = if platform.uses_command_key() {
            self.mac_cmd || self.ctrl
        } else {
            false
        };
        let expected_ctrl = if platform.uses_command_key() {
            if self.mac_cmd {
                false
            } else {
                self.ctrl
            }
        } else {
            self.ctrl || self.mac_cmd
        };

        let cmd_match = if platform.uses_command_key() {
            egui_mods.mac_cmd == expected_cmd
        } else {
            true
        };

        cmd_match
            && egui_mods.ctrl == expected_ctrl
            && egui_mods.alt == self.alt
            && egui_mods.shift == self.shift
    }

    pub fn to_display_string(&self, platform: Platform) -> String {
        let mut parts = Vec::new();
        if platform.uses_command_key() {
            if self.ctrl {
                parts.push("Control");
            }
            if self.alt {
                parts.push("Option");
            }
            if self.shift {
                parts.push("Shift");
            }
            if self.mac_cmd {
                parts.push("Cmd");
            }
        } else {
            if self.ctrl || self.mac_cmd {
                parts.push("Ctrl");
            }
            if self.alt {
                parts.push("Alt");
            }
            if self.shift {
                parts.push("Shift");
            }
        }
        parts.join("+")
    }
}

/// Abstract representation of key values for cross-platform shortcut definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Escape,
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Equals,
    Minus,
    Plus,
    Slash,
}

impl Key {
    pub fn to_egui_key(&self) -> Option<egui::Key> {
        match self {
            Key::A => Some(egui::Key::A),
            Key::B => Some(egui::Key::B),
            Key::C => Some(egui::Key::C),
            Key::D => Some(egui::Key::D),
            Key::E => Some(egui::Key::E),
            Key::F => Some(egui::Key::F),
            Key::G => Some(egui::Key::G),
            Key::H => Some(egui::Key::H),
            Key::I => Some(egui::Key::I),
            Key::J => Some(egui::Key::J),
            Key::K => Some(egui::Key::K),
            Key::L => Some(egui::Key::L),
            Key::M => Some(egui::Key::M),
            Key::N => Some(egui::Key::N),
            Key::O => Some(egui::Key::O),
            Key::P => Some(egui::Key::P),
            Key::Q => Some(egui::Key::Q),
            Key::R => Some(egui::Key::R),
            Key::S => Some(egui::Key::S),
            Key::T => Some(egui::Key::T),
            Key::U => Some(egui::Key::U),
            Key::V => Some(egui::Key::V),
            Key::W => Some(egui::Key::W),
            Key::X => Some(egui::Key::X),
            Key::Y => Some(egui::Key::Y),
            Key::Z => Some(egui::Key::Z),
            Key::Num0 => Some(egui::Key::Num0),
            Key::Num1 => Some(egui::Key::Num1),
            Key::Num2 => Some(egui::Key::Num2),
            Key::Num3 => Some(egui::Key::Num3),
            Key::Num4 => Some(egui::Key::Num4),
            Key::Num5 => Some(egui::Key::Num5),
            Key::Num6 => Some(egui::Key::Num6),
            Key::Num7 => Some(egui::Key::Num7),
            Key::Num8 => Some(egui::Key::Num8),
            Key::Num9 => Some(egui::Key::Num9),
            Key::F1 => Some(egui::Key::F1),
            Key::F2 => Some(egui::Key::F2),
            Key::F3 => Some(egui::Key::F3),
            Key::F4 => Some(egui::Key::F4),
            Key::F5 => Some(egui::Key::F5),
            Key::F6 => Some(egui::Key::F6),
            Key::F7 => Some(egui::Key::F7),
            Key::F8 => Some(egui::Key::F8),
            Key::F9 => Some(egui::Key::F9),
            Key::F10 => Some(egui::Key::F10),
            Key::F11 => Some(egui::Key::F11),
            Key::F12 => Some(egui::Key::F12),
            Key::Escape => Some(egui::Key::Escape),
            Key::Enter => Some(egui::Key::Enter),
            Key::Tab => Some(egui::Key::Tab),
            Key::Space => Some(egui::Key::Space),
            Key::Backspace => Some(egui::Key::Backspace),
            Key::Delete => Some(egui::Key::Delete),
            Key::Home => Some(egui::Key::Home),
            Key::End => Some(egui::Key::End),
            Key::PageUp => Some(egui::Key::PageUp),
            Key::PageDown => Some(egui::Key::PageDown),
            Key::ArrowUp => Some(egui::Key::ArrowUp),
            Key::ArrowDown => Some(egui::Key::ArrowDown),
            Key::ArrowLeft => Some(egui::Key::ArrowLeft),
            Key::ArrowRight => Some(egui::Key::ArrowRight),
            Key::Equals => Some(egui::Key::Equals),
            Key::Minus => Some(egui::Key::Minus),
            Key::Plus => Some(egui::Key::Plus),
            Key::Slash => Some(egui::Key::Slash),
        }
    }

    pub fn from_egui_key(key: egui::Key) -> Option<Self> {
        match key {
            egui::Key::A => Some(Key::A),
            egui::Key::B => Some(Key::B),
            egui::Key::C => Some(Key::C),
            egui::Key::D => Some(Key::D),
            egui::Key::E => Some(Key::E),
            egui::Key::F => Some(Key::F),
            egui::Key::G => Some(Key::G),
            egui::Key::H => Some(Key::H),
            egui::Key::I => Some(Key::I),
            egui::Key::J => Some(Key::J),
            egui::Key::K => Some(Key::K),
            egui::Key::L => Some(Key::L),
            egui::Key::M => Some(Key::M),
            egui::Key::N => Some(Key::N),
            egui::Key::O => Some(Key::O),
            egui::Key::P => Some(Key::P),
            egui::Key::Q => Some(Key::Q),
            egui::Key::R => Some(Key::R),
            egui::Key::S => Some(Key::S),
            egui::Key::T => Some(Key::T),
            egui::Key::U => Some(Key::U),
            egui::Key::V => Some(Key::V),
            egui::Key::W => Some(Key::W),
            egui::Key::X => Some(Key::X),
            egui::Key::Y => Some(Key::Y),
            egui::Key::Z => Some(Key::Z),
            egui::Key::Num0 => Some(Key::Num0),
            egui::Key::Num1 => Some(Key::Num1),
            egui::Key::Num2 => Some(Key::Num2),
            egui::Key::Num3 => Some(Key::Num3),
            egui::Key::Num4 => Some(Key::Num4),
            key_other => match key_other {
                egui::Key::Num5 => Some(Key::Num5),
                egui::Key::Num6 => Some(Key::Num6),
                egui::Key::Num7 => Some(Key::Num7),
                egui::Key::Num8 => Some(Key::Num8),
                egui::Key::Num9 => Some(Key::Num9),
                egui::Key::F1 => Some(Key::F1),
                egui::Key::F2 => Some(Key::F2),
                egui::Key::F3 => Some(Key::F3),
                egui::Key::F4 => Some(Key::F4),
                egui::Key::F5 => Some(Key::F5),
                egui::Key::F6 => Some(Key::F6),
                egui::Key::F7 => Some(Key::F7),
                egui::Key::F8 => Some(Key::F8),
                egui::Key::F9 => Some(Key::F9),
                egui::Key::F10 => Some(Key::F10),
                egui::Key::F11 => Some(Key::F11),
                egui::Key::F12 => Some(Key::F12),
                egui::Key::Escape => Some(Key::Escape),
                egui::Key::Enter => Some(Key::Enter),
                egui::Key::Tab => Some(Key::Tab),
                egui::Key::Space => Some(Key::Space),
                egui::Key::Backspace => Some(Key::Backspace),
                egui::Key::Delete => Some(Key::Delete),
                egui::Key::Home => Some(Key::Home),
                egui::Key::End => Some(Key::End),
                egui::Key::PageUp => Some(Key::PageUp),
                egui::Key::PageDown => Some(Key::PageDown),
                egui::Key::ArrowUp => Some(Key::ArrowUp),
                egui::Key::ArrowDown => Some(Key::ArrowDown),
                egui::Key::ArrowLeft => Some(Key::ArrowLeft),
                egui::Key::ArrowRight => Some(Key::ArrowRight),
                egui::Key::Equals => Some(Key::Equals),
                egui::Key::Minus => Some(Key::Minus),
                egui::Key::Plus => Some(Key::Plus),
                egui::Key::Slash => Some(Key::Slash),
                _ => None,
            },
        }
    }

    pub fn to_display_name(&self) -> &'static str {
        match self {
            Key::A => "A",
            Key::B => "B",
            Key::C => "C",
            Key::D => "D",
            Key::E => "E",
            Key::F => "F",
            Key::G => "G",
            Key::H => "H",
            Key::I => "I",
            Key::J => "J",
            Key::K => "K",
            Key::L => "L",
            Key::M => "M",
            Key::N => "N",
            Key::O => "O",
            Key::P => "P",
            Key::Q => "Q",
            Key::R => "R",
            Key::S => "S",
            Key::T => "T",
            Key::U => "U",
            Key::V => "V",
            Key::W => "W",
            Key::X => "X",
            Key::Y => "Y",
            Key::Z => "Z",
            Key::Num0 => "0",
            Key::Num1 => "1",
            Key::Num2 => "2",
            Key::Num3 => "3",
            Key::Num4 => "4",
            Key::Num5 => "5",
            Key::Num6 => "6",
            Key::Num7 => "7",
            Key::Num8 => "8",
            Key::Num9 => "9",
            Key::F1 => "F1",
            Key::F2 => "F2",
            Key::F3 => "F3",
            Key::F4 => "F4",
            Key::F5 => "F5",
            Key::F6 => "F6",
            Key::F7 => "F7",
            Key::F8 => "F8",
            Key::F9 => "F9",
            Key::F10 => "F10",
            Key::F11 => "F11",
            Key::F12 => "F12",
            Key::Escape => "Esc",
            Key::Enter => "Enter",
            Key::Tab => "Tab",
            Key::Space => "Space",
            Key::Backspace => "Backspace",
            Key::Delete => "Del",
            Key::Home => "Home",
            Key::End => "End",
            Key::PageUp => "PageUp",
            Key::PageDown => "PageDown",
            Key::ArrowUp => "Up",
            Key::ArrowDown => "Down",
            Key::ArrowLeft => "Left",
            Key::ArrowRight => "Right",
            Key::Equals => "=",
            Key::Minus => "-",
            Key::Plus => "+",
            Key::Slash => "/",
        }
    }
}

/// A complete key chord combination consisting of modifiers and a primary key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyChord {
    pub modifiers: Modifiers,
    pub key: Key,
}

impl KeyChord {
    pub const fn new(modifiers: Modifiers, key: Key) -> Self {
        Self { modifiers, key }
    }

    pub const fn key(key: Key) -> Self {
        Self {
            modifiers: Modifiers::NONE,
            key,
        }
    }

    pub const fn ctrl(key: Key) -> Self {
        Self {
            modifiers: Modifiers::CTRL,
            key,
        }
    }

    pub const fn alt(key: Key) -> Self {
        Self {
            modifiers: Modifiers::ALT,
            key,
        }
    }

    pub const fn shift(key: Key) -> Self {
        Self {
            modifiers: Modifiers::SHIFT,
            key,
        }
    }

    pub const fn cmd(key: Key) -> Self {
        Self {
            modifiers: Modifiers::CMD,
            key,
        }
    }

    pub const fn ctrl_shift(key: Key) -> Self {
        Self {
            modifiers: Modifiers::CTRL_SHIFT,
            key,
        }
    }

    pub fn to_display_string(&self, platform: Platform) -> String {
        let mod_str = self.modifiers.to_display_string(platform);
        let key_str = self.key.to_display_name();
        if mod_str.is_empty() {
            key_str.to_string()
        } else {
            format!("{}+{}", mod_str, key_str)
        }
    }
}

/// Central registry mapping key combinations to spreadsheet actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutRegistry {
    pub platform: Platform,
    bindings: HashMap<KeyChord, ShortcutAction>,
    primary_chords: HashMap<ShortcutAction, KeyChord>,
}

impl ShortcutRegistry {
    pub fn new(platform: Platform) -> Self {
        let mut registry = Self {
            platform,
            bindings: HashMap::new(),
            primary_chords: HashMap::new(),
        };
        registry.populate_defaults();
        registry
    }

    pub fn default_for_current_platform() -> Self {
        Self::new(Platform::current())
    }

    pub fn new_windows() -> Self {
        Self::new(Platform::Windows)
    }

    pub fn new_linux() -> Self {
        Self::new(Platform::Linux)
    }

    pub fn new_macos() -> Self {
        Self::new(Platform::MacOS)
    }

    pub fn bind(&mut self, action: ShortcutAction, chord: KeyChord) {
        self.bindings.insert(chord, action);
        self.primary_chords.insert(action, chord);
    }

    pub fn unbind(&mut self, chord: &KeyChord) {
        if let Some(action) = self.bindings.remove(chord) {
            if self.primary_chords.get(&action) == Some(chord) {
                // Find alternative chord for action if exists
                if let Some((alt_chord, _)) = self.bindings.iter().find(|(_, a)| **a == action) {
                    self.primary_chords.insert(action, *alt_chord);
                } else {
                    self.primary_chords.remove(&action);
                }
            }
        }
    }

    pub fn get_action(&self, chord: &KeyChord) -> Option<ShortcutAction> {
        self.bindings.get(chord).copied()
    }

    pub fn primary_shortcut_for(&self, action: ShortcutAction) -> Option<KeyChord> {
        self.primary_chords.get(&action).copied()
    }

    pub fn shortcut_text(&self, action: ShortcutAction) -> String {
        self.primary_shortcut_for(action)
            .map(|chord| chord.to_display_string(self.platform))
            .unwrap_or_default()
    }

    /// Evaluates egui input state and returns the triggered action, if any key was pressed.
    pub fn matches_input(&self, input: &egui::InputState) -> Option<ShortcutAction> {
        // Iterate through registered shortcuts
        for (chord, action) in &self.bindings {
            if let Some(egui_key) = chord.key.to_egui_key() {
                if input.key_pressed(egui_key)
                    && chord
                        .modifiers
                        .matches_egui(&input.modifiers, self.platform)
                {
                    return Some(*action);
                }
            }
        }
        None
    }

    /// Checks if a specific action was just pressed in this input frame.
    pub fn is_action_pressed(&self, action: ShortcutAction, input: &egui::InputState) -> bool {
        for (chord, mapped_action) in &self.bindings {
            if *mapped_action == action {
                if let Some(egui_key) = chord.key.to_egui_key() {
                    if input.key_pressed(egui_key)
                        && chord
                            .modifiers
                            .matches_egui(&input.modifiers, self.platform)
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn populate_defaults(&mut self) {
        let primary_mod = if self.platform.uses_command_key() {
            Modifiers::CMD
        } else {
            Modifiers::CTRL
        };

        let primary_shift = if self.platform.uses_command_key() {
            Modifiers::CMD_SHIFT
        } else {
            Modifiers::CTRL_SHIFT
        };

        // File operations
        self.bind(
            ShortcutAction::NewWorkbook,
            KeyChord::new(primary_mod, Key::N),
        );
        self.bind(
            ShortcutAction::OpenWorkbook,
            KeyChord::new(primary_mod, Key::O),
        );
        self.bind(
            ShortcutAction::SaveWorkbook,
            KeyChord::new(primary_mod, Key::S),
        );
        self.bind(
            ShortcutAction::SaveAsWorkbook,
            KeyChord::new(primary_shift, Key::S),
        );

        // History & Clipboard
        self.bind(ShortcutAction::Undo, KeyChord::new(primary_mod, Key::Z));
        self.bind(ShortcutAction::Redo, KeyChord::new(primary_mod, Key::Y));
        if self.platform.uses_command_key() {
            // Secondary Redo on Mac: Cmd+Shift+Z
            self.bindings.insert(
                KeyChord::new(Modifiers::CMD_SHIFT, Key::Z),
                ShortcutAction::Redo,
            );
        }
        self.bind(ShortcutAction::Cut, KeyChord::new(primary_mod, Key::X));
        self.bind(ShortcutAction::Copy, KeyChord::new(primary_mod, Key::C));
        self.bind(ShortcutAction::Paste, KeyChord::new(primary_mod, Key::V));

        // Basic Navigation
        self.bind(
            ShortcutAction::MoveCursor(Direction::Up),
            KeyChord::key(Key::ArrowUp),
        );
        self.bind(
            ShortcutAction::MoveCursor(Direction::Down),
            KeyChord::key(Key::ArrowDown),
        );
        self.bind(
            ShortcutAction::MoveCursor(Direction::Left),
            KeyChord::key(Key::ArrowLeft),
        );
        self.bind(
            ShortcutAction::MoveCursor(Direction::Right),
            KeyChord::key(Key::ArrowRight),
        );

        // Extended Selection (Shift + Arrows)
        self.bind(
            ShortcutAction::ExtendSelection(Direction::Up),
            KeyChord::shift(Key::ArrowUp),
        );
        self.bind(
            ShortcutAction::ExtendSelection(Direction::Down),
            KeyChord::shift(Key::ArrowDown),
        );
        self.bind(
            ShortcutAction::ExtendSelection(Direction::Left),
            KeyChord::shift(Key::ArrowLeft),
        );
        self.bind(
            ShortcutAction::ExtendSelection(Direction::Right),
            KeyChord::shift(Key::ArrowRight),
        );

        // Jump Navigation (Ctrl + Arrows)
        self.bind(
            ShortcutAction::JumpCursor(Direction::Up),
            KeyChord::new(primary_mod, Key::ArrowUp),
        );
        self.bind(
            ShortcutAction::JumpCursor(Direction::Down),
            KeyChord::new(primary_mod, Key::ArrowDown),
        );
        self.bind(
            ShortcutAction::JumpCursor(Direction::Left),
            KeyChord::new(primary_mod, Key::ArrowLeft),
        );
        self.bind(
            ShortcutAction::JumpCursor(Direction::Right),
            KeyChord::new(primary_mod, Key::ArrowRight),
        );

        // Extended Jump Navigation (Ctrl + Shift + Arrows)
        self.bind(
            ShortcutAction::ExtendJumpSelection(Direction::Up),
            KeyChord::new(primary_shift, Key::ArrowUp),
        );
        self.bind(
            ShortcutAction::ExtendJumpSelection(Direction::Down),
            KeyChord::new(primary_shift, Key::ArrowDown),
        );
        self.bind(
            ShortcutAction::ExtendJumpSelection(Direction::Left),
            KeyChord::new(primary_shift, Key::ArrowLeft),
        );
        self.bind(
            ShortcutAction::ExtendJumpSelection(Direction::Right),
            KeyChord::new(primary_shift, Key::ArrowRight),
        );

        // Selection & Edge jumps
        self.bind(
            ShortcutAction::SelectAll,
            KeyChord::new(primary_mod, Key::A),
        );
        self.bind(
            ShortcutAction::GoToHome,
            KeyChord::new(primary_mod, Key::Home),
        );
        self.bind(
            ShortcutAction::GoToEnd,
            KeyChord::new(primary_mod, Key::End),
        );
        self.bind(ShortcutAction::PageUp, KeyChord::key(Key::PageUp));
        self.bind(ShortcutAction::PageDown, KeyChord::key(Key::PageDown));

        // Cell Editing
        self.bind(ShortcutAction::EditCell, KeyChord::key(Key::F2));
        self.bind(ShortcutAction::Cancel, KeyChord::key(Key::Escape));
        self.bind(ShortcutAction::CommitAndMoveDown, KeyChord::key(Key::Enter));
        self.bind(ShortcutAction::CommitAndMoveUp, KeyChord::shift(Key::Enter));
        self.bind(ShortcutAction::CommitAndMoveRight, KeyChord::key(Key::Tab));
        self.bind(ShortcutAction::CommitAndMoveLeft, KeyChord::shift(Key::Tab));
        self.bind(
            ShortcutAction::ToggleAbsoluteReference,
            KeyChord::key(Key::F4),
        ); // F4 repeat / abs toggle
        self.bind(ShortcutAction::AutoSum, KeyChord::alt(Key::Equals)); // Alt+= AutoSum
        self.bind(ShortcutAction::ClearContents, KeyChord::key(Key::Delete));
        self.bind(ShortcutAction::Backspace, KeyChord::key(Key::Backspace));

        // Formatting
        self.bind(
            ShortcutAction::FormatCells,
            KeyChord::new(primary_mod, Key::Num1),
        );
        self.bind(ShortcutAction::Bold, KeyChord::new(primary_mod, Key::B));
        self.bind(ShortcutAction::Italic, KeyChord::new(primary_mod, Key::I));
        self.bind(
            ShortcutAction::Underline,
            KeyChord::new(primary_mod, Key::U),
        );
        self.bind(ShortcutAction::Find, KeyChord::new(primary_mod, Key::F));
        self.bind(ShortcutAction::Replace, KeyChord::new(primary_mod, Key::H));
        self.bind(ShortcutAction::Recalculate, KeyChord::key(Key::F9));
    }
}

/// Cycles a cell reference or formula token between relative and absolute modes (F4 shortcut functionality).
/// Cycles through: A1 -> $A$1 -> A$1 -> $A1 -> A1
pub fn toggle_cell_reference_abs_rel(ref_str: &str) -> String {
    let trimmed = ref_str.trim();
    if trimmed.is_empty() {
        return ref_str.to_string();
    }

    let mut col_part = String::new();
    let mut row_part = String::new();
    let mut col_abs = false;
    let mut row_abs = false;

    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0;

    // Check optional leading $
    if i < chars.len() && chars[i] == '$' {
        col_abs = true;
        i += 1;
    }

    // Read column letters
    while i < chars.len() && chars[i].is_ascii_alphabetic() {
        col_part.push(chars[i]);
        i += 1;
    }

    if col_part.is_empty() {
        return ref_str.to_string();
    }

    // Check optional row $
    if i < chars.len() && chars[i] == '$' {
        row_abs = true;
        i += 1;
    }

    // Read row digits
    while i < chars.len() && chars[i].is_ascii_digit() {
        row_part.push(chars[i]);
        i += 1;
    }

    if row_part.is_empty() || i < chars.len() {
        // Not a standard single cell reference
        return ref_str.to_string();
    }

    // Standard cycle:
    // (col_abs, row_abs)
    // (false, false) -> (true, true)   : A1 -> $A$1
    // (true, true)   -> (false, true)  : $A$1 -> A$1
    // (false, true)  -> (true, false)  : A$1 -> $A1
    // (true, false)  -> (false, false) : $A1 -> A1
    let (next_col_abs, next_row_abs) = match (col_abs, row_abs) {
        (false, false) => (true, true),
        (true, true) => (false, true),
        (false, true) => (true, false),
        (true, false) => (false, false),
    };

    let mut result = String::new();
    if next_col_abs {
        result.push('$');
    }
    result.push_str(&col_part);
    if next_row_abs {
        result.push('$');
    }
    result.push_str(&row_part);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_shortcuts_windows() {
        let registry = ShortcutRegistry::new_windows();
        assert_eq!(registry.platform, Platform::Windows);

        let copy_chord = registry.primary_shortcut_for(ShortcutAction::Copy).unwrap();
        assert_eq!(copy_chord.key, Key::C);
        assert!(copy_chord.modifiers.ctrl);
        assert!(!copy_chord.modifiers.mac_cmd);
        assert_eq!(copy_chord.to_display_string(Platform::Windows), "Ctrl+C");

        let autosum_chord = registry
            .primary_shortcut_for(ShortcutAction::AutoSum)
            .unwrap();
        assert_eq!(autosum_chord.key, Key::Equals);
        assert!(autosum_chord.modifiers.alt);
        assert_eq!(autosum_chord.to_display_string(Platform::Windows), "Alt+=");

        let edit_chord = registry
            .primary_shortcut_for(ShortcutAction::EditCell)
            .unwrap();
        assert_eq!(edit_chord.key, Key::F2);
        assert_eq!(edit_chord.to_display_string(Platform::Windows), "F2");

        let f4_chord = registry
            .primary_shortcut_for(ShortcutAction::ToggleAbsoluteReference)
            .unwrap();
        assert_eq!(f4_chord.key, Key::F4);
    }

    #[test]
    fn test_platform_shortcuts_macos() {
        let registry = ShortcutRegistry::new_macos();
        assert_eq!(registry.platform, Platform::MacOS);

        let copy_chord = registry.primary_shortcut_for(ShortcutAction::Copy).unwrap();
        assert_eq!(copy_chord.key, Key::C);
        assert!(copy_chord.modifiers.mac_cmd);
        assert_eq!(copy_chord.to_display_string(Platform::MacOS), "Cmd+C");

        let redo_chord = registry.primary_shortcut_for(ShortcutAction::Redo).unwrap();
        assert_eq!(redo_chord.key, Key::Y);
    }

    #[test]
    fn test_toggle_cell_reference_f4() {
        assert_eq!(toggle_cell_reference_abs_rel("A1"), "$A$1");
        assert_eq!(toggle_cell_reference_abs_rel("$A$1"), "A$1");
        assert_eq!(toggle_cell_reference_abs_rel("A$1"), "$A1");
        assert_eq!(toggle_cell_reference_abs_rel("$A1"), "A1");
        assert_eq!(toggle_cell_reference_abs_rel("XFD1048576"), "$XFD$1048576");
    }
}
