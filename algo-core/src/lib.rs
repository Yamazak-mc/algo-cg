use anyhow::{bail, Context as _};
use rand::seq::SliceRandom as _;
use std::collections::BTreeMap;
use tracing::{debug, info};

pub mod settings;
use settings::GameSettings;

pub mod card;
use card::{CardNumber, CardView, Talon};

pub mod player;
use player::{Player, PlayerId, PlayerView, TurnPlayer};

pub mod event;
use event::{EventQueue, GameEvent};

/// A structure controlling the game sequences.
#[derive(Debug, Clone)]
pub struct Game {
    // static
    settings: GameSettings,

    // board state
    board: Board,

    // turn management
    turn: TurnContext,

    // event management
    staged_event: Option<GameEvent>,
    event_queue: EventQueue<GameEvent>,
    event_responses: BTreeMap<PlayerId, Option<GameEvent>>,
    history: Vec<GameEvent>,
}

impl Game {
    /// Creates a game with 2 players.
    ///
    /// Returns `Err` if:
    /// - the provided 2 PlayerIds are the same
    /// - the settings are invalid
    ///
    /// This function decides the turn order and shuffles the deck randomly
    /// using `ThreadRng`.
    pub fn for_2_players(
        player_ids: (PlayerId, PlayerId),
        settings: GameSettings,
    ) -> anyhow::Result<Self> {
        if player_ids.0 == player_ids.1 {
            bail!("duplicated PlayerId: {:?}", player_ids.0);
        }

        let mut rng = rand::rng();

        let mut talon: Talon = settings.clone().build_cards()?.into_iter().collect();
        if talon.len() <= settings.initial_draw_num as usize * 2 {
            bail!("invalid game settings: not enough cards to start the game");
        }
        talon.shuffle(&mut rng);

        let players = BTreeMap::from([
            (player_ids.0, Player::default()),
            (player_ids.1, Player::default()),
        ]);

        let turn_order = {
            let mut order = [player_ids.0, player_ids.1];
            order.shuffle(&mut rng);
            TurnPlayer::new(order)
        };

        let event_queue = EventQueue {
            main_queue: [GameEvent::GameStarted(talon.view())].into(),
            ..Default::default()
        };

        let event_responses = players.keys().map(|id| (*id, None)).collect();

        let ret = Self {
            settings,
            board: Board::new(talon, players),
            turn: TurnContext::new(turn_order),
            staged_event: None,
            event_queue,
            event_responses,
            history: Vec::new(),
        };
        Ok(ret)
    }

    /// Provides board information from the perspective of the specified player.
    ///
    /// Returns `Err` if the specified PlayerId is unknown.
    pub fn view_board(&self, viewer: PlayerId) -> anyhow::Result<BoardView> {
        self.board.view(viewer)
    }

    /// Starts processing the next [`GameEvent`].
    pub fn next_event(
        &mut self,
    ) -> Result<impl Iterator<Item = (PlayerId, GameEvent)> + '_, NextEventError> {
        if self.is_event_staged() {
            return Err(NextEventError::EventProcessing);
        }
        let Some(event) = self.event_queue.pop_next() else {
            return Err(NextEventError::NoMoreEvent);
        };

        self.start_event(event.clone());

        let ret = self
            .event_responses
            .keys()
            .map(move |id| (*id, event.view(*id)));
        Ok(ret)
    }

    /// Stores a `GameEvent` response received from a player client.
    /// If the responding player was the last one to respond,
    /// the [`process_event`] will be available and returns `Ok(true)`
    ///
    /// Returns `Ok(false)` if there are still players have not responded yet.
    ///
    /// Returns `Err` if:
    /// - The specified PlayerId is invalid
    /// - `process_event` returns an error
    ///
    /// [`process_event`]: `Game::process_event`
    pub fn store_player_response(
        &mut self,
        player: PlayerId,
        response: GameEvent,
    ) -> anyhow::Result<bool> {
        let storage = self
            .event_responses
            .get_mut(&player)
            .context("unknown PlayerId")?;

        *storage = Some(response);

        Ok(self.has_all_players_responded())
    }

    fn turn_order(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.turn.turn_player.turn_order()
    }

    fn is_event_staged(&self) -> bool {
        self.staged_event.is_some()
    }

    fn has_all_players_responded(&self) -> bool {
        self.event_responses.values().all(Option::is_some)
    }

    fn start_event(&mut self, event: GameEvent) {
        self.staged_event = Some(event);
    }

    pub fn process_event(&mut self) -> Result<(), ProcessEventError> {
        if !self.has_all_players_responded() {
            return Err(ProcessEventError::NotReady);
        }

        // DEBUG
        {
            debug!("process event: starting");
            debug!("responses:");
            for (pid, resp) in self.event_responses.iter() {
                debug!("- {:?}: {:?}", pid, resp.as_ref().unwrap());
            }
        }

        // TODO: Implement event logic

        // Cleanup
        self.finish_event();

        Ok(())
    }

    fn finish_event(&mut self) {
        self.history.push(
            self.staged_event
                .take()
                .expect("there should be a staged event"),
        );

        // Cleanup responses
        for val in self.event_responses.values_mut() {
            val.take();
        }
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum NextEventError {
    #[error("the event is processing")]
    EventProcessing,
    #[error("there is no more events to process")]
    NoMoreEvent,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProcessEventError {
    #[error("the event is not ready to be processed")]
    NotReady,
    #[error("failed to process the event")]
    Failed,
}

/// Board information
#[derive(Debug, Clone, PartialEq, Eq)]
struct Board {
    talon: Talon,
    players: BTreeMap<PlayerId, Player>,
}

impl Board {
    fn new(talon: Talon, players: BTreeMap<PlayerId, Player>) -> Self {
        Self { talon, players }
    }

    // TODO: Accept guests(non-players) as viewer somehow.
    fn view(&self, viewer: PlayerId) -> anyhow::Result<BoardView> {
        let myself = self
            .players
            .get(&viewer)
            .context(format!("unknown PlayerId {:?}", viewer))?
            .clone();

        let other_players = self
            .players
            .iter()
            .filter(|(k, _)| **k != viewer)
            .map(|(k, v)| (*k, v.public_view()))
            .collect();

        let ret = BoardView {
            myself,
            other_players,
            talon_remaining: self.talon.len() as u32,
            talon_top: self.talon.view_top(),
        };
        Ok(ret)
    }
}

/// Board information to provide to players.
pub struct BoardView {
    pub myself: Player,
    pub other_players: BTreeMap<PlayerId, PlayerView>,
    pub talon_remaining: u32,
    pub talon_top: Option<CardView>,
}

/// Turn context.
///
/// A turn consists of one or more attack sessions.
#[derive(Debug, Clone)]
struct TurnContext {
    turn_player: TurnPlayer,
    attack: AttackContext,
}

impl TurnContext {
    fn new(turn_player: TurnPlayer) -> Self {
        Self {
            turn_player,
            attack: AttackContext::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct AttackContext {
    target_player: Option<PlayerId>,
    target_card_idx: Option<u32>,
    guess: Option<CardNumber>,
}
