use anyhow::{bail, Context as _};
use rand::seq::SliceRandom as _;
use std::{collections::BTreeMap, fmt};
use tracing::debug;

// TODO: fix visibility

pub mod settings;
use settings::GameSettings;

pub mod card;
use card::{CardNumber, Talon};

pub mod player;
use player::{Player, PlayerId, TurnPlayer};

pub mod event;
use event::{BoardChange, CardLocation, CardMovement, EventQueue, GameEvent, GameEventKind};

type ProcessEventResult = Result<(), ProcessEventError>;

/// A structure controlling the game sequences.
#[derive(Debug, Clone)]
pub struct Game {
    // static
    settings: GameSettings,

    // board state
    board: Board,

    // turn management
    turn_player: TurnPlayer,
    attack: AttackContext,

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
            turn_player: turn_order.clone(),
            attack: AttackContext::default(),
            staged_event: None,
            event_queue,
            event_responses,
            history: Vec::new(),
        };
        Ok(ret)
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

    pub fn process_event(&mut self) -> ProcessEventResult {
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

        // Main logic
        let event = self.staged_event.take().unwrap();
        match event {
            GameEvent::BoardChanged(_) => (),
            GameEvent::GameStarted(_) => {
                self.start_game();
            }
            GameEvent::TurnOrderDetermined(_) => (),
            GameEvent::CardDistributed(pid) => {
                let change = self.board.draw_direct(pid);
                self.event_queue.push_sub(GameEvent::BoardChanged(change));
            }
            GameEvent::TurnStarted(_) => {
                self.start_turn();
                self.event_queue.push_main(GameEvent::TurnPlayerDrewCard);
            }
            GameEvent::TurnPlayerDrewCard => {
                self.resolve_turn_player_draw();
            }
            GameEvent::NoCardsLeft => {
                self.event_queue.push_main(GameEvent::GameEnded);
            }
            GameEvent::AttackTargetSelectionRequired { .. } => {
                self.resolve_resp_attack_target_selection()?;
            }
            GameEvent::AttackTargetSelected { .. } => {
                self.event_queue.push_main(GameEvent::NumberGuessRequired);
            }
            GameEvent::NumberGuessRequired => {
                self.resolve_resp_number_guess()?;
            }
            GameEvent::NumberGuessed(_) => {
                self.resolve_attack();
            }
            GameEvent::AttackSucceeded => {
                self.resolve_succeeded_attack();
            }
            GameEvent::AttackFailed => {
                self.resolve_failed_attack();
            }
            GameEvent::AttackedPlayerLost => {
                // Currently, there are only 2 players
                // If one loses, the game should end immediately.
                self.event_queue.push_main(GameEvent::GameEnded);
            }
            GameEvent::GameEnded => (),
            GameEvent::AttackOrStayDecisionRequired => {
                self.resolve_resp_attack_or_stay_decision()?;
            }
            GameEvent::AttackOrStayDecided { attack } => {
                if attack {
                    self.event_queue
                        .push_main(GameEvent::AttackTargetSelectionRequired {
                            target_player: self.attack_target_player(),
                        });
                } else {
                    self.resolve_stay();
                }
            }
            GameEvent::TurnEnded => {
                self.end_turn();

                self.event_queue
                    .push_main(GameEvent::TurnStarted(self.turn_player()));
            }
            GameEvent::RespOk => unreachable!(),
        }

        // Responses left in the storage are expected to be `RespOk`.
        for (pid, resp) in self.event_responses.iter_mut() {
            match resp.take() {
                Some(GameEvent::RespOk) => {
                    continue;
                }
                Some(unexpected) => {
                    return Err(ResponseError {
                        kind: ResponseErrorKind::InvalidGameEventKind {
                            expected: GameEventKind::RespOk,
                        },
                        player: *pid,
                        response: unexpected,
                    }
                    .into());
                }
                None => unreachable!(),
            }
        }

        // Update history
        self.history.push(event);

        Ok(())
    }

    fn is_event_staged(&self) -> bool {
        self.staged_event.is_some()
    }

    fn has_all_players_responded(&self) -> bool {
        self.event_responses.values().all(Option::is_some)
    }

    fn turn_order(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.turn_player.turn_order()
    }

    fn turn_player(&self) -> PlayerId {
        self.turn_player.get().unwrap()
    }

    fn attack_target_player(&self) -> PlayerId {
        self.attack.target_player.unwrap()
    }

    fn start_event(&mut self, event: GameEvent) {
        self.staged_event = Some(event);
    }

    fn take_turn_player_resp(&mut self) -> GameEvent {
        self.event_responses
            .get_mut(&self.turn_player())
            .unwrap()
            .take()
            .unwrap()
    }

    fn start_game(&mut self) {
        let turn_order = self.turn_order().collect::<Vec<_>>();

        self.event_queue
            .push_main(GameEvent::TurnOrderDetermined(turn_order.clone()));

        for _ in 0..self.settings.initial_draw_num {
            for pid in &turn_order {
                self.event_queue.push_main(GameEvent::CardDistributed(*pid));
            }
        }
        self.event_queue
            .push_main(GameEvent::TurnStarted(self.turn_player()));
    }

    fn start_turn(&mut self) {
        let attack_target = {
            let mut players = self.turn_player.clone();
            players.advance();
            players.get().unwrap()
        };

        self.attack.target_player = Some(attack_target);
    }

    fn resolve_turn_player_draw(&mut self) {
        let draw_res = self.board.draw(self.turn_player());

        match draw_res {
            Some(change) => {
                self.event_queue.push_sub(GameEvent::BoardChanged(change));

                self.event_queue
                    .push_main(GameEvent::AttackTargetSelectionRequired {
                        target_player: self.attack_target_player(),
                    });
            }
            None => self.event_queue.push_main(GameEvent::NoCardsLeft),
        }
    }

    fn resolve_resp_attack_target_selection(&mut self) -> ProcessEventResult {
        let resp = self.take_turn_player_resp();
        let GameEvent::AttackTargetSelected { target_idx } = resp else {
            return Err(self.invalid_resp_kind(GameEventKind::AttackTargetSelected, resp));
        };

        if !self
            .board
            .verify_attack_target(self.attack_target_player(), target_idx)
        {
            return Err(self.resp_err(ResponseErrorKind::InvalidAttackTarget, resp));
        }

        self.attack.target_card_idx = Some(target_idx);
        self.event_queue.push_main(resp);

        Ok(())
    }

    fn resolve_resp_number_guess(&mut self) -> ProcessEventResult {
        let resp = self.take_turn_player_resp();
        let GameEvent::NumberGuessed(num) = resp else {
            return Err(self.invalid_resp_kind(GameEventKind::NumberGuessed, resp));
        };

        if !(0..=11).contains(&num.0) {
            return Err(self.resp_err(ResponseErrorKind::NumberOutOfRange, resp));
        }

        self.attack.guess = Some(num);
        self.event_queue.push_main(resp);

        Ok(())
    }

    fn resolve_attack(&mut self) {
        let attacked = self.attack_target_player();
        let target_idx = self.attack.target_card_idx.unwrap();
        let guess = self.attack.guess.unwrap();

        let res = self.board.resolve_attack(attacked, target_idx, guess);

        self.event_queue.push_main(if res {
            GameEvent::AttackSucceeded
        } else {
            GameEvent::AttackFailed
        });
    }

    fn resolve_succeeded_attack(&mut self) {
        let attacked = self.attack_target_player();
        let target_idx = self.attack.target_card_idx.unwrap();

        let change = self.board.resolve_succeeded_attack(attacked, target_idx);
        self.event_queue.push_sub(GameEvent::BoardChanged(change));

        let res = self.board.has_player_lost_game(attacked);
        self.event_queue.push_main(if res {
            GameEvent::AttackedPlayerLost
        } else {
            GameEvent::AttackOrStayDecisionRequired
        });

        // Cleanup
        self.attack.target_card_idx.take();
        self.attack.guess.take();
    }

    fn resolve_failed_attack(&mut self) {
        let attacker = self.turn_player();

        for change in self.board.resolve_failed_attack(attacker) {
            self.event_queue.push_sub(GameEvent::BoardChanged(change));
        }

        self.event_queue.push_main(GameEvent::TurnEnded);
    }

    fn resolve_resp_attack_or_stay_decision(&mut self) -> ProcessEventResult {
        let resp = self.take_turn_player_resp();
        let GameEvent::AttackOrStayDecided { .. } = resp else {
            return Err(self.invalid_resp_kind(GameEventKind::AttackOrStayDecided, resp));
        };

        self.event_queue.push_main(resp);

        Ok(())
    }

    fn resolve_stay(&mut self) {
        let change = self.board.resolve_stay(self.turn_player());

        self.event_queue.push_sub(GameEvent::BoardChanged(change));
        self.event_queue.push_main(GameEvent::TurnEnded);
    }

    fn end_turn(&mut self) {
        self.turn_player.advance();

        self.attack.cleanup();
    }

    fn resp_err(&self, kind: ResponseErrorKind, resp: GameEvent) -> ProcessEventError {
        ResponseError {
            kind,
            player: self.turn_player(),
            response: resp,
        }
        .into()
    }

    fn invalid_resp_kind(&self, kind: GameEventKind, resp: GameEvent) -> ProcessEventError {
        self.resp_err(
            ResponseErrorKind::InvalidGameEventKind { expected: kind },
            resp,
        )
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
    #[error(transparent)]
    ResponseError(#[from] ResponseError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseError {
    pub kind: ResponseErrorKind,
    pub player: PlayerId,
    pub response: GameEvent,
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ResponseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseErrorKind {
    InvalidGameEventKind { expected: GameEventKind },
    InvalidAttackTarget,
    NumberOutOfRange,
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

    fn draw_direct(&mut self, player: PlayerId) -> BoardChange {
        let card = self.talon.draw().expect("talon should have some cards");

        let idx = self
            .players
            .get_mut(&player)
            .unwrap()
            .insert_card_to_field(card);

        BoardChange::CardMoved {
            player,
            movement: CardMovement::TalonToField { insert_at: idx },
            card: card.full_view(),
        }
    }

    fn draw(&mut self, player: PlayerId) -> Option<BoardChange> {
        let card = self.talon.draw()?;

        self.players.get_mut(&player).unwrap().insert_attacker(card);

        Some(BoardChange::CardMoved {
            player,
            movement: CardMovement::TalonToAttacker,
            card: card.full_view(),
        })
    }

    fn verify_attack_target(&self, target_player: PlayerId, target_idx: u32) -> bool {
        let Some(card) = self
            .players
            .get(&target_player)
            .unwrap()
            .field
            .get(target_idx as usize)
        else {
            return false;
        };

        !card.pub_info.revealed
    }

    /// Returns `true` if guess is correct.
    fn resolve_attack(&mut self, attacked: PlayerId, target_idx: u32, guess: CardNumber) -> bool {
        let attacked_card = self.players.get(&attacked).unwrap().field[target_idx as usize];
        guess == attacked_card.priv_info.number
    }

    fn resolve_succeeded_attack(&mut self, attacked: PlayerId, target_idx: u32) -> BoardChange {
        let attacked_card = self
            .players
            .get_mut(&attacked)
            .unwrap()
            .field
            .get_mut(target_idx as usize)
            .unwrap();

        attacked_card.pub_info.revealed = true;

        BoardChange::CardRevealed {
            player: attacked,
            location: CardLocation::Field { idx: target_idx },
            card: *attacked_card,
        }
    }

    fn has_player_lost_game(&mut self, player: PlayerId) -> bool {
        self.players
            .get(&player)
            .unwrap()
            .field
            .iter()
            .all(|v| v.pub_info.revealed)
    }

    fn resolve_failed_attack(&mut self, attacker: PlayerId) -> [BoardChange; 2] {
        let player = self.players.get_mut(&attacker).unwrap();

        let mut attacker_card = player.attacker.take().unwrap();
        attacker_card.pub_info.revealed = true;

        let idx = player.insert_card_to_field(attacker_card);

        [
            BoardChange::CardRevealed {
                player: attacker,
                location: CardLocation::Attacker,
                card: attacker_card,
            },
            BoardChange::CardMoved {
                player: attacker,
                movement: CardMovement::AttackerToField { insert_at: idx },
                card: attacker_card.full_view(),
            },
        ]
    }

    fn resolve_stay(&mut self, attacker: PlayerId) -> BoardChange {
        let player = self.players.get_mut(&attacker).unwrap();

        let attacker_card = player.attacker.take().unwrap();

        let idx = player.insert_card_to_field(attacker_card);

        BoardChange::CardMoved {
            player: attacker,
            movement: CardMovement::AttackerToField { insert_at: idx },
            card: attacker_card.full_view(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct AttackContext {
    target_player: Option<PlayerId>,
    target_card_idx: Option<u32>,
    guess: Option<CardNumber>,
}

impl AttackContext {
    fn cleanup(&mut self) {
        *self = Self::default();
    }
}
