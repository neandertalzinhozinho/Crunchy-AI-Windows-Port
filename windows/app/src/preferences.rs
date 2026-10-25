use std::{fs, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preferences {
    pub microphone: Option<String>,
    pub light_model: bool,
    pub mode: usize,
    pub strength: u32,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            microphone: None,
            light_model: false,
            mode: 0,
            strength: 40,
        }
    }
}

impl Preferences {
    pub fn load() -> Self {
        let Some(path) = preferences_path() else {
            return Self::default();
        };
        Self::load_from(&path)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = preferences_path() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "A pasta de configurações do usuário não está disponível",
            ));
        };
        self.save_to(&path)
    }

    fn load_from(path: &std::path::Path) -> Self {
        let Ok(contents) = fs::read_to_string(path) else {
            return Self::default();
        };
        Self::parse(&contents)
    }

    fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.serialize())
    }

    fn parse(contents: &str) -> Self {
        let mut preferences = Self::default();
        for line in contents.lines() {
            let Some((key, value)) = line.trim().split_once('=') else {
                continue;
            };
            match key {
                "microphone" => preferences.microphone = decode_hex(value),
                "light_model" => preferences.light_model = value == "true",
                "mode" => {
                    if let Ok(mode @ 0..=2) = value.parse() {
                        preferences.mode = mode;
                    }
                }
                "strength" => {
                    if let Ok(strength @ 0..=100) = value.parse() {
                        preferences.strength = strength;
                    }
                }
                _ => {}
            }
        }
        preferences
    }

    fn serialize(&self) -> String {
        let microphone = self
            .microphone
            .as_deref()
            .map(encode_hex)
            .unwrap_or_default();
        format!(
            "microphone={microphone}\nlight_model={}\nmode={}\nstrength={}\n",
            self.light_model, self.mode, self.strength
        )
    }
}

fn preferences_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("LOCALAPPDATA"))
        .map(PathBuf::from)
        .map(|root| root.join("HushMic").join("preferences.ini"))
}

fn encode_hex(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode_hex(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::Preferences;
    use std::{fs, path::PathBuf};

    fn temp_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "hushmic-preferences-test-{}-{}.ini",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn round_trips_unicode_device_name_and_settings() {
        let value = Preferences {
            microphone: Some("Microfone USB 🎙".into()),
            light_model: true,
            mode: 2,
            strength: 73,
        };
        assert_eq!(Preferences::parse(&value.serialize()), value);
    }

    #[test]
    fn invalid_values_keep_safe_defaults() {
        let value = Preferences::parse(
            "microphone=ff\nlight_model=yes\nmode=9\nstrength=1000\nunknown=ignored\n",
        );
        assert_eq!(value, Preferences::default());
    }

    #[test]
    fn empty_microphone_round_trips_as_unselected() {
        let value = Preferences::default();
        assert_eq!(Preferences::parse(&value.serialize()), value);
    }

    #[test]
    fn saves_and_loads_preferences_from_disk() {
        let path = temp_path();
        let value = Preferences {
            microphone: Some("USB 🎙".into()),
            light_model: true,
            mode: 1,
            strength: 65,
        };
        value.save_to(&path).unwrap();
        assert_eq!(Preferences::load_from(&path), value);
        fs::remove_file(path).unwrap();
    }
}
