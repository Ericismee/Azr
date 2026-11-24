#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Search,
    Launchpad,
    Overview,
}

impl Command {
    pub const ALL: [Self; 3] = [Self::Search, Self::Launchpad, Self::Overview];

    pub const fn index(self) -> usize {
        match self {
            Self::Search => 0,
            Self::Launchpad => 1,
            Self::Overview => 2,
        }
    }

    pub const fn default_chord(self, launchpad_key: u8) -> Chord {
        match self {
            Self::Search => Chord {
                modifiers: ALT,
                key: 0x20,
            },
            Self::Launchpad if launchpad_key == 2 => Chord {
                modifiers: CTRL | ALT,
                key: b'L' as u16,
            },
            Self::Launchpad => Chord {
                modifiers: CTRL | SHIFT,
                key: 0x20,
            },
            Self::Overview => Chord {
                modifiers: CTRL,
                key: 0x26,
            },
        }
    }
}

pub const ALT: u8 = 1;
pub const CTRL: u8 = 2;
pub const SHIFT: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub modifiers: u8,
    pub key: u16,
}

impl Chord {
    pub fn new(modifiers: u8, key: u16) -> Option<Self> {
        if modifiers == 0
            || modifiers & !(CTRL | ALT | SHIFT) != 0
            || !matches!(
                key, 0x08 | 0x09 | 0x0D | 0x1B | 0x20..=0x28 | 0x2D..=0x2E | 0x30..=0x39
                    | 0x41..=0x5A | 0x70..=0x7A | 0x7C..=0x87 | 0xBA..=0xC0 | 0xDB..=0xDE
            )
        {
            return None;
        }
        if (modifiers & ALT != 0 && matches!(key, 0x09 | 0x1B | 0x73))
            || (key == 0x1B && modifiers & (CTRL | SHIFT) == CTRL | SHIFT)
            || (key == 0x2E && modifiers & (CTRL | ALT) == CTRL | ALT)
            || (key == 0xC0 && modifiers & ALT != 0)
        {
            return None;
        }
        Some(Self { modifiers, key })
    }

    pub fn parse(value: &str) -> Option<Self> {
        let (mods, key) = value.split_once(',')?;
        Self::new(mods.parse().ok()?, key.parse().ok()?)
    }

    pub fn encode(self) -> String {
        format!("{},{}", self.modifiers, self.key)
    }

    pub fn label(self) -> String {
        let mut parts = Vec::new();
        for (mask, name) in [(CTRL, "Ctrl"), (ALT, "Alt"), (SHIFT, "Shift")] {
            if self.modifiers & mask != 0 {
                parts.push(name.to_owned());
            }
        }
        parts.push(match self.key {
            0x08 => "Backspace".into(),
            0x09 => "Tab".into(),
            0x0D => "Enter".into(),
            0x1B => "Esc".into(),
            0x20 => "Space".into(),
            0x21 => "Page Up".into(),
            0x22 => "Page Down".into(),
            0x23 => "End".into(),
            0x24 => "Home".into(),
            0x25 => "←".into(),
            0x26 => "↑".into(),
            0x27 => "→".into(),
            0x28 => "↓".into(),
            0x2D => "Insert".into(),
            0x2E => "Delete".into(),
            0x30..=0x39 | 0x41..=0x5A => (self.key as u8 as char).to_string(),
            0x70..=0x87 => format!("F{}", self.key - 0x6F),
            0xBA..=0xC0 => [";", "=", ",", "-", ".", "/", "`"][usize::from(self.key - 0xBA)].into(),
            0xDB..=0xDE => ["[", "\\", "]", "'"][usize::from(self.key - 0xDB)].into(),
            _ => format!("VK {:02X}", self.key),
        });
        parts.join(" + ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_alt_space_and_round_trips() {
        let chord = Chord::new(ALT, 0x20).unwrap();
        assert_eq!(chord.label(), "Alt + Space");
        assert_eq!(Chord::parse(&chord.encode()), Some(chord));
    }

    #[test]
    fn preserves_windows_and_existing_azr_system_shortcuts() {
        for (mods, key) in [
            (ALT, 9),
            (ALT, 0x73),
            (CTRL | SHIFT, 0x1B),
            (CTRL | ALT, 0x2E),
            (ALT, 0xC0),
            (0, b'K' as u16),
        ] {
            assert_eq!(Chord::new(mods, key), None);
        }
        assert_eq!(Chord::parse("2,255"), None);
    }
}
