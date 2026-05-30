use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rand::seq::SliceRandom;
use std::time::{Duration, Instant};

use crate::audio::DeviceInfo;
use crate::config::StoredConfig;
use crate::music::{Key, Note, drill_pool};
use crate::pitch::PitchEvent;

pub const REQUIRED_HITS: u8 = 4;
pub const CELEBRATION_DURATION: Duration = Duration::from_millis(1000);

pub enum Screen {
    DevicePicker {
        devices: Vec<(cpal::Device, DeviceInfo)>,
        cursor: usize,
    },
    ChannelPicker {
        device: cpal::Device,
        device_name: String,
        channels: u16,
        cursor: usize,
    },
    KeyPicker {
        device: cpal::Device,
        device_name: String,
        channel: u16,
        keys: Vec<Key>,
        cursor: usize,
    },
    Drill {
        device_name: String,
        channel: u16,
        key: Key,
        pool: Vec<Note>,
        state: DrillState,
        last_detected: Option<PitchEvent>,
    },
    Transitioning,
}

pub enum DrillState {
    Listening {
        target: Note,
        consecutive_hits: u8,
    },
    Celebrating {
        note: Note,
        expires_at: Instant,
    },
}

pub enum Transition {
    Stay,
    Quit,
    StartCapture {
        device: cpal::Device,
        device_name: String,
        channel: u16,
        key: Key,
    },
}

pub struct App {
    pub screen: Screen,
    pub config: StoredConfig,
    pub status: Option<String>,
}

impl App {
    pub fn new(
        devices: Vec<(cpal::Device, DeviceInfo)>,
        config: StoredConfig,
    ) -> Self {
        let cursor = config
            .device_name
            .as_deref()
            .and_then(|name| devices.iter().position(|(_, info)| info.name == name))
            .unwrap_or(0);
        Self {
            screen: Screen::DevicePicker { devices, cursor },
            config,
            status: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Transition {
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            return Transition::Quit;
        }
        match std::mem::replace(&mut self.screen, Screen::Transitioning) {
            Screen::DevicePicker { mut devices, mut cursor } => {
                let mut next = Transition::Stay;
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if cursor > 0 {
                            cursor -= 1;
                        }
                        self.screen = Screen::DevicePicker { devices, cursor };
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if cursor + 1 < devices.len() {
                            cursor += 1;
                        }
                        self.screen = Screen::DevicePicker { devices, cursor };
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        next = Transition::Quit;
                        self.screen = Screen::DevicePicker { devices, cursor };
                    }
                    KeyCode::Enter => {
                        if devices.is_empty() {
                            self.screen = Screen::DevicePicker { devices, cursor };
                        } else {
                            let (device, info) = devices.swap_remove(cursor);
                            let device_name = info.name.clone();
                            self.config.device_name = Some(device_name.clone());
                            let channel_cursor = self
                                .config
                                .channel
                                .and_then(|c| (c < info.max_input_channels).then_some(c as usize))
                                .unwrap_or(0);
                            self.screen = Screen::ChannelPicker {
                                device,
                                device_name,
                                channels: info.max_input_channels,
                                cursor: channel_cursor,
                            };
                        }
                    }
                    _ => {
                        self.screen = Screen::DevicePicker { devices, cursor };
                    }
                }
                next
            }
            Screen::ChannelPicker {
                device,
                device_name,
                channels,
                mut cursor,
            } => {
                let mut next = Transition::Stay;
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if cursor > 0 {
                            cursor -= 1;
                        }
                        self.screen = Screen::ChannelPicker {
                            device,
                            device_name,
                            channels,
                            cursor,
                        };
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if cursor + 1 < (channels as usize) {
                            cursor += 1;
                        }
                        self.screen = Screen::ChannelPicker {
                            device,
                            device_name,
                            channels,
                            cursor,
                        };
                    }
                    KeyCode::Char('q') => {
                        next = Transition::Quit;
                        self.screen = Screen::ChannelPicker {
                            device,
                            device_name,
                            channels,
                            cursor,
                        };
                    }
                    KeyCode::Esc => {
                        drop(device);
                        self.go_back_to_device_picker();
                    }
                    KeyCode::Enter => {
                        let chan = cursor as u16;
                        self.config.channel = Some(chan);
                        let keys = Key::all();
                        let key_cursor = self
                            .config
                            .key()
                            .and_then(|k| keys.iter().position(|x| *x == k))
                            .unwrap_or(0);
                        self.screen = Screen::KeyPicker {
                            device,
                            device_name,
                            channel: chan,
                            keys,
                            cursor: key_cursor,
                        };
                    }
                    _ => {
                        self.screen = Screen::ChannelPicker {
                            device,
                            device_name,
                            channels,
                            cursor,
                        };
                    }
                }
                next
            }
            Screen::KeyPicker {
                device,
                device_name,
                channel,
                keys,
                mut cursor,
            } => {
                let mut next = Transition::Stay;
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if cursor > 0 {
                            cursor -= 1;
                        }
                        self.screen = Screen::KeyPicker {
                            device,
                            device_name,
                            channel,
                            keys,
                            cursor,
                        };
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if cursor + 1 < keys.len() {
                            cursor += 1;
                        }
                        self.screen = Screen::KeyPicker {
                            device,
                            device_name,
                            channel,
                            keys,
                            cursor,
                        };
                    }
                    KeyCode::Char('q') => {
                        next = Transition::Quit;
                        self.screen = Screen::KeyPicker {
                            device,
                            device_name,
                            channel,
                            keys,
                            cursor,
                        };
                    }
                    KeyCode::Esc => {
                        drop(device);
                        self.go_back_to_device_picker();
                    }
                    KeyCode::Enter => {
                        let selected_key = keys[cursor];
                        self.config.set_key(selected_key);
                        next = Transition::StartCapture {
                            device,
                            device_name,
                            channel,
                            key: selected_key,
                        };
                    }
                    _ => {
                        self.screen = Screen::KeyPicker {
                            device,
                            device_name,
                            channel,
                            keys,
                            cursor,
                        };
                    }
                }
                next
            }
            Screen::Drill {
                device_name,
                channel,
                key: musical_key,
                pool,
                state,
                last_detected,
            } => {
                let next = match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => Transition::Quit,
                    _ => Transition::Stay,
                };
                self.screen = Screen::Drill {
                    device_name,
                    channel,
                    key: musical_key,
                    pool,
                    state,
                    last_detected,
                };
                next
            }
            Screen::Transitioning => {
                self.screen = Screen::Transitioning;
                Transition::Stay
            }
        }
    }

    pub fn reset_to_device_picker(&mut self) {
        self.go_back_to_device_picker();
    }

    fn go_back_to_device_picker(&mut self) {
        let devices = crate::audio::list_input_devices().unwrap_or_default();
        let cursor = self
            .config
            .device_name
            .as_deref()
            .and_then(|name| devices.iter().position(|(_, info)| info.name == name))
            .unwrap_or(0);
        self.screen = Screen::DevicePicker { devices, cursor };
    }

    pub fn enter_drill(&mut self, device_name: String, channel: u16, key: Key) {
        self.screen = build_drill_screen(device_name, channel, key);
    }

    pub fn on_pitch_event(&mut self, event: PitchEvent) {
        if let Screen::Drill {
            state,
            last_detected,
            ..
        } = &mut self.screen
        {
            *last_detected = Some(event);
            match state {
                DrillState::Listening {
                    target,
                    consecutive_hits,
                } => {
                    if event.note.midi == target.midi {
                        *consecutive_hits = consecutive_hits.saturating_add(1);
                        if *consecutive_hits >= REQUIRED_HITS {
                            *state = DrillState::Celebrating {
                                note: *target,
                                expires_at: Instant::now() + CELEBRATION_DURATION,
                            };
                        }
                    } else {
                        *consecutive_hits = 0;
                    }
                }
                DrillState::Celebrating { .. } => {}
            }
        }
    }

    pub fn tick(&mut self) {
        if let Screen::Drill {
            pool,
            state,
            last_detected,
            ..
        } = &mut self.screen
        {
            if let DrillState::Celebrating { note, expires_at } = state {
                if Instant::now() >= *expires_at {
                    let next = pick_next_note(pool, Some(*note));
                    *state = DrillState::Listening {
                        target: next,
                        consecutive_hits: 0,
                    };
                    *last_detected = None;
                }
            }
        }
    }
}

pub fn pick_next_note(pool: &[Note], avoid: Option<Note>) -> Note {
    let mut rng = rand::thread_rng();
    let candidates: Vec<&Note> = pool
        .iter()
        .filter(|n| avoid.map_or(true, |a| a.midi != n.midi))
        .collect();
    let pick = if candidates.is_empty() {
        pool.choose(&mut rng).expect("non-empty pool")
    } else {
        *candidates.choose(&mut rng).expect("non-empty filtered pool")
    };
    *pick
}

pub fn build_drill_screen(device_name: String, channel: u16, key: Key) -> Screen {
    let pool = drill_pool(key);
    let first = pick_next_note(&pool, None);
    Screen::Drill {
        device_name,
        channel,
        key,
        pool,
        state: DrillState::Listening {
            target: first,
            consecutive_hits: 0,
        },
        last_detected: None,
    }
}
