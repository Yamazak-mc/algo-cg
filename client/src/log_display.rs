use bevy::prelude::*;
use std::collections::VecDeque;

// TODO: make LogDisplay scrollable

pub fn log_display_plugin(app: &mut App) {
    app.add_event::<LogEvent>()
        .add_systems(Update, (relay_event, handle_log_display).chain());
}

#[derive(Debug, Clone)]
pub struct LogDisplaySettings {
    pub max_lines: usize,
    pub font: TextFont,
}

impl Default for LogDisplaySettings {
    fn default() -> Self {
        Self {
            max_lines: 20,
            font: TextFont::default()
        }
    }
}

#[derive(Component)]
#[require(Text)]
pub struct LogDisplay {
    settings: LogDisplaySettings,
    _start_idx: usize,
    logs: Vec<Message>,
    events: VecDeque<LogEvent>,
    lines: Vec<Option<Entity>>,
}

impl LogDisplay {
    pub fn new(settings: LogDisplaySettings) -> Self {
        let lines = vec![None; settings.max_lines];

        Self {
            settings,
            _start_idx: 0,
            logs: Vec::new(),
            events: VecDeque::new(),
            lines,
        }
    }

    pub fn with_message(mut self, message: Message) -> Self {
        self.push(message);
        self
    }

    pub fn with_messages(mut self, messages: impl IntoIterator<Item = Message>) -> Self {
        self.extend(messages);
        self
    }

    pub fn clear(&mut self) {
        if matches!(self.events.front(), Some(LogEvent::Clear)) {
            self.events.drain(1..);
        } else {
            self.events.push_back(LogEvent::Clear);
        }
    }

    pub fn push(&mut self, message: Message) {
        self.events.extend(message.normalized().map(LogEvent::Push));
    }

    pub fn extend(&mut self, messages: impl IntoIterator<Item = Message>) {
        self.events.extend(
            messages
                .into_iter()
                .flat_map(|v| v.normalized().collect::<Vec<_>>())
                .map(LogEvent::Push),
        );
    }

    fn update(&mut self, self_id: Entity, world_cmds: &mut Commands) {
        if self.events.is_empty() {
            return;
        }

        let prev_len = self.logs.len();
        let has_clear_cmd = self.events.front().unwrap().is_clear();

        for log_cmd in self.events.drain(..) {
            match log_cmd {
                LogEvent::Clear => self.logs.clear(),
                LogEvent::Push(msg) => self.logs.push(msg),
            }
        }

        let len = self.logs.len();
        let max_lines = self.settings.max_lines;
        let start = len.saturating_sub(max_lines);
        let stop = (start + max_lines).min(len);

        for (msg, entity) in self.logs[start..stop].iter().zip(&mut self.lines) {
            let components = (msg.to_components(), self.settings.font.clone());

            if let Some(entity) = entity {
                world_cmds.entity(*entity).insert(components);
                continue;
            }

            world_cmds.entity(self_id).with_children(|parent| {
                let id = parent.spawn(components).id();
                *entity = Some(id);
            });
        }

        if has_clear_cmd && prev_len >= stop {
            for entity in self.lines.iter().skip(stop) {
                let Some(entity) = entity else {
                    continue;
                };

                world_cmds.entity(*entity).insert(TextSpan::new("\n"));
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Event)]
pub enum LogEvent {
    Clear,
    Push(Message),
}

impl LogEvent {
    fn is_clear(&self) -> bool {
        matches!(self, Self::Clear)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Message {
    pub text: String,
    pub color: Color,
}

impl Message {
    pub fn new(message: impl Into<String>, color: Color) -> Self {
        Self {
            text: message.into(),
            color,
        }
    }

    pub fn from_text(message: impl Into<String>) -> Self {
        Self {
            text: message.into(),
            ..default()
        }
    }

    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Color::srgb(0.0, 1.0, 0.0),
        }
    }

    pub fn warn(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Color::srgb(1.0, 1.0, 0.0),
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Color::srgb(1.0, 0.0, 0.0),
        }
    }

    pub fn debug(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Color::srgb_u8(0x77, 0xCD, 0xFF),
        }
    }

    fn normalized(&self) -> impl Iterator<Item = Self> + '_ {
        let color = self.color;

        self.text.lines().map(move |line| Self {
            text: line.into(),
            color,
        })
    }

    fn to_components(&self) -> impl Bundle {
        (
            TextSpan(format!("{}\n", self.text)),
            TextColor(self.color),
            ViewVisibility::default(),
        )
    }
}

fn handle_log_display(mut commands: Commands, mut query: Query<(Entity, &mut LogDisplay)>) {
    for (entity, mut log_display) in &mut query {
        log_display.update(entity, &mut commands);
    }
}

fn relay_event(
    mut ev_reader: EventMutator<LogEvent>,
    log_display: Option<Single<&mut LogDisplay>>,
) {
    let Some(mut log_display) = log_display else {
        return;
    };

    for ev in ev_reader.read() {
        match ev {
            LogEvent::Clear => log_display.clear(),
            LogEvent::Push(msg) => {
                let text = std::mem::take(&mut msg.text);
                log_display.push(Message {
                    text,
                    color: msg.color,
                });
            }
        }
    }
}
