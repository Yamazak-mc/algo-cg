use bevy::{input::mouse::MouseScrollUnit, prelude::*};
use std::collections::VecDeque;

use super::scrollable::ScrollEvent;

// TODO: make LogDisplay scrollable

pub fn log_display_plugin(app: &mut App) {
    app.add_event::<LogEvent>()
        .add_systems(Update, (relay_event, handle_log_display).chain())
        .add_observer(LogDisplay::on_add_log_display);
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
            font: TextFont::default(),
        }
    }
}

#[derive(Component)]
#[require(Text)]
pub struct LogDisplay {
    settings: LogDisplaySettings,
    logs: Vec<Message>,
    events: VecDeque<LogEvent>,
    lines: Vec<Option<Entity>>,
    scroll: usize,
    prev_scroll: usize,
}

impl LogDisplay {
    pub fn new(settings: LogDisplaySettings) -> Self {
        let lines = vec![None; settings.max_lines];

        Self {
            settings,
            logs: Vec::new(),
            events: VecDeque::new(),
            lines,
            scroll: 0,
            prev_scroll: 0,
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

    pub fn debug(&self) {
        info!("===LogDisplay===");
        for log in &self.logs {
            info!("{} {}", log.header(), log.text);
        }
        info!("================");
    }

    pub fn extend(&mut self, messages: impl IntoIterator<Item = Message>) {
        self.events.extend(
            messages
                .into_iter()
                .flat_map(|v| v.normalized().collect::<Vec<_>>())
                .map(LogEvent::Push),
        );
    }

    fn on_add_log_display(trigger: Trigger<OnAdd, Self>, mut commands: Commands) {
        commands.entity(trigger.entity()).observe(Self::on_scroll);
    }

    pub fn queue_scroll(&mut self, lines: i32) {
        if lines < 0 {
            self.scroll = self.scroll.saturating_sub(lines.unsigned_abs() as usize);
        } else {
            let len = self.logs.len();
            let max_lines = self.settings.max_lines;
            if len > max_lines {
                self.scroll = (self.scroll + lines as usize).min(len - max_lines);
            }
        }
    }

    fn on_scroll(trigger: Trigger<ScrollEvent>, mut query: Query<&mut LogDisplay>) {
        let event = trigger.event().0;

        let (_dx, dy) = match event.unit {
            MouseScrollUnit::Line => (event.x, event.y),
            MouseScrollUnit::Pixel => {
                warn!("unsupported MouseScrollUnit for LogDisplay: Pixel");
                return; // TODO
            }
        };

        query
            .get_mut(trigger.entity())
            .unwrap()
            .queue_scroll(dy as i32);
    }

    fn update(&mut self, self_id: Entity, world_cmds: &mut Commands) {
        if self.events.is_empty() && self.scroll == self.prev_scroll {
            return;
        }
        self.prev_scroll = self.scroll;

        let prev_len = self.logs.len();
        let has_clear_cmd = match self.events.front() {
            Some(ev) => ev.is_clear(),
            None => false,
        };

        for log_cmd in self.events.drain(..) {
            match log_cmd {
                LogEvent::Clear => self.logs.clear(),
                LogEvent::Push(msg) => {
                    #[cfg(not(debug_assertions))]
                    {
                        if msg.level == Some(MessageLevel::Debug) {
                            continue;
                        }
                    }
                    self.logs.push(msg)
                }
                _ => (),
            }
        }

        let len = self.logs.len();
        let max_lines = self.settings.max_lines;
        let mut start = len.saturating_sub(max_lines);

        if self.scroll > 0 && len > max_lines {
            start = start.saturating_sub(self.scroll);
        }

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
    Debug,
    Scroll(i32),
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
    pub level: Option<MessageLevel>,
}

impl Message {
    pub fn new(message: impl Into<String>, color: Color) -> Self {
        Self {
            text: message.into(),
            color,
            ..default()
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            text: message.into(),
            level: Some(MessageLevel::Info),
            ..default()
        }
    }

    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Color::srgb(0.0, 1.0, 0.0),
            level: Some(MessageLevel::Success),
        }
    }

    pub fn warn(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Color::srgb(1.0, 1.0, 0.0),
            level: Some(MessageLevel::Warn),
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Color::srgb(1.0, 0.0, 0.0),
            level: Some(MessageLevel::Error),
        }
    }

    pub fn debug(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Color::srgb_u8(0x77, 0xCD, 0xFF),
            level: Some(MessageLevel::Debug),
        }
    }

    fn normalized(&self) -> impl Iterator<Item = Self> + '_ {
        let color = self.color;

        self.text.lines().map(move |line| Self {
            text: line.into(),
            color,
            level: self.level,
        })
    }

    fn to_components(&self) -> impl Bundle {
        (
            TextSpan(format!("{}\n", self.text)),
            TextColor(self.color),
            ViewVisibility::default(),
            PickingBehavior::IGNORE,
        )
    }

    fn header(&self) -> String {
        let level = self.level.map_or(default(), |v| v.as_str());
        format!("{:>07}", level.to_uppercase())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    Debug,
    Info,
    Success,
    Warn,
    Error,
}

impl MessageLevel {
    fn as_str(&self) -> &'static str {
        match self {
            MessageLevel::Debug => "DEBUG",
            MessageLevel::Info => "INFO",
            MessageLevel::Success => "SUCCESS",
            MessageLevel::Warn => "WARN",
            MessageLevel::Error => "ERROR",
        }
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
                    level: msg.level,
                });
            }
            LogEvent::Debug => {
                log_display.debug();
            }
            LogEvent::Scroll(dy) => {
                log_display.queue_scroll(*dy);
            }
        }
    }
}
