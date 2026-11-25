use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Normalized input to individual game cartridges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameInput {
    Up,
    Down,
    Left,
    Right,
    Action,
    RotateCcw,
    ContinueEndless,
}

/// High-level console events decoded from terminal events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleEvent {
    Resize(u16, u16),
    ForceQuit,
    Up,
    Down,
    Left,
    Right,
    Confirm,
    ActionSpace,
    BackOrPause,
    PauseToggle,
    Restart,
    QuitOrMenu,
    OpenSettings,
    OpenHighScores,
    CycleTheme,
    CycleDifficulty,
    RotateCcw,
    ContinueEndless,
    Number(u8),
    AnyKey,
}

/// Translates a `crossterm::event::Event` into a platform-consistent `ConsoleEvent`.
/// Filters out key-release events so Windows Terminal and Kitty enhanced protocols
/// never double-trigger actions.
pub fn decode_event(event: Event) -> Option<ConsoleEvent> {
    match event {
        Event::Resize(w, h) => Some(ConsoleEvent::Resize(w, h)),
        Event::Key(key) => decode_key_event(key),
        _ => None,
    }
}

pub fn decode_key_event(key: KeyEvent) -> Option<ConsoleEvent> {
    if key.kind == KeyEventKind::Release {
        return None;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL)
        && let KeyCode::Char('c' | 'C' | 'q' | 'Q') = key.code
    {
        return Some(ConsoleEvent::ForceQuit);
    }

    let ev = match key.code {
        KeyCode::Up => ConsoleEvent::Up,
        KeyCode::Down => ConsoleEvent::Down,
        KeyCode::Left => ConsoleEvent::Left,
        KeyCode::Right => ConsoleEvent::Right,
        KeyCode::Enter => ConsoleEvent::Confirm,
        KeyCode::Char(' ') => ConsoleEvent::ActionSpace,
        KeyCode::Esc => ConsoleEvent::BackOrPause,
        KeyCode::Tab | KeyCode::BackTab => ConsoleEvent::OpenSettings,
        KeyCode::Char(ch) => match ch.to_ascii_lowercase() {
            'w' | 'k' => ConsoleEvent::Up,
            's' | 'j' => ConsoleEvent::Down,
            'a' => ConsoleEvent::Left,
            'd' => ConsoleEvent::Right,
            'p' => ConsoleEvent::PauseToggle,
            'r' => ConsoleEvent::Restart,
            'q' | 'm' => ConsoleEvent::QuitOrMenu,
            'c' => ConsoleEvent::ContinueEndless,
            'o' => ConsoleEvent::OpenSettings,
            'h' => ConsoleEvent::OpenHighScores,
            't' => ConsoleEvent::CycleTheme,
            'f' => ConsoleEvent::CycleDifficulty,
            'z' => ConsoleEvent::RotateCcw,
            'x' => ConsoleEvent::ActionSpace,
            '1'..='6' => ConsoleEvent::Number(ch as u8 - b'0'),
            _ => ConsoleEvent::AnyKey,
        },
        _ => ConsoleEvent::AnyKey,
    };

    Some(ev)
}

impl ConsoleEvent {
    /// Converts a console event into a gameplay input when a cartridge is actively running.
    pub fn to_game_input(self) -> Option<GameInput> {
        match self {
            Self::Up => Some(GameInput::Up),
            Self::Down => Some(GameInput::Down),
            Self::Left => Some(GameInput::Left),
            Self::Right => Some(GameInput::Right),
            Self::ActionSpace | Self::Confirm => Some(GameInput::Action),
            Self::RotateCcw => Some(GameInput::RotateCcw),
            Self::ContinueEndless => Some(GameInput::ContinueEndless),
            _ => None,
        }
    }
}
