use crate::{InboundEvent, OutboundEvent};
use algo_core::{
    player::{AssignPlayerId, PlayerId},
    settings::GameSettings,
    Game, NextEventError,
};
use anyhow::{bail, Context as _};
use protocol::{server_to_client::JoinInfo, WithMetadata};
use std::collections::BTreeMap;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::{debug, info, warn};

mod player_handler;
use player_handler::PlayerHandler;

#[derive(Debug, Clone)]
pub enum ServerInternalEvent {
    // inbound
    In(PlayerId, WithMetadata<InboundEvent>),
    RequestJoin(UnboundedSender<Self>),
    ConnectionLost(PlayerId),

    // outbound
    Out(WithMetadata<OutboundEvent>),
    RequestJoinAccepted(JoinInfo),
}

#[derive(Debug)]
pub struct WaitingRoom {
    rx: UnboundedReceiver<ServerInternalEvent>,
}

impl WaitingRoom {
    pub fn new(rx: UnboundedReceiver<ServerInternalEvent>) -> Self {
        Self { rx }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let mut player_handlers = BTreeMap::<_, PlayerHandler>::new();
        let mut room = WaitingRoomSeats::default();
        let mut new_player_id = AssignPlayerId::default();

        while !room.is_full() {
            let Some(ev) = self.rx.recv().await else {
                bail!("server internal error: channel closed");
            };
            match ev {
                ServerInternalEvent::RequestJoin(tx) => {
                    let player_id = new_player_id.assign();

                    let join_info = room.try_claim(player_id)?;
                    tx.send(ServerInternalEvent::RequestJoinAccepted(join_info))?;

                    // Notify that the new player joined the server to waiting players.
                    for handler in player_handlers.values_mut() {
                        handler.send_message(OutboundEvent::PlayerJoined(join_info))?;
                    }

                    player_handlers.insert(player_id, PlayerHandler::new(tx));
                }
                ServerInternalEvent::ConnectionLost(player_id) => {
                    info!("player {:?} left the waiting room", player_id);

                    room.remove(player_id);
                    player_handlers.remove(&player_id);
                }
                unexpected => {
                    warn!("unexpected event: {:?}", unexpected);
                }
            }
        }

        let player_ids = {
            let mut keys = player_handlers.keys().cloned();
            (keys.next().unwrap(), keys.next().unwrap())
        };

        let game = Game::for_2_players(player_ids, GameSettings::default())?;
        GameInstance::new(self.rx, game, player_handlers)
            .run()
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum WaitingRoomSeats {
    #[default]
    Empty,
    One(PlayerId),
    Two(PlayerId, PlayerId),
}

impl WaitingRoomSeats {
    fn player_num(&self) -> u8 {
        match self {
            Self::Empty => 0,
            Self::One(_) => 1,
            Self::Two(..) => 2,
        }
    }

    fn room_size(&self) -> u8 {
        2
    }

    fn is_full(&self) -> bool {
        matches!(self, Self::Two(..))
    }

    fn try_claim(&mut self, new_player: PlayerId) -> anyhow::Result<JoinInfo> {
        let room = match self {
            Self::Empty => Self::One(new_player),
            Self::One(id) => Self::Two(*id, new_player),
            Self::Two(..) => bail!("room is full"),
        };

        *self = room;

        let ret = JoinInfo {
            player_id: new_player,
            join_position: self.player_num(),
            room_size: self.room_size(),
        };
        Ok(ret)
    }

    fn remove(&mut self, player: PlayerId) {
        match self {
            Self::Empty => (),
            Self::One(player_id) if *player_id == player => {
                *self = Self::Empty;
            }
            Self::Two(a, b) => {
                if *a == player {
                    *self = Self::One(*b);
                } else if *b == player {
                    *self = Self::One(*a);
                }
            }
            _ => (),
        }
    }
}

struct GameInstance {
    rx: UnboundedReceiver<ServerInternalEvent>,
    game: Game,
    player_handlers: BTreeMap<PlayerId, PlayerHandler>,
}

impl GameInstance {
    fn new(
        rx: UnboundedReceiver<ServerInternalEvent>,
        game: Game,
        player_handlers: BTreeMap<PlayerId, PlayerHandler>,
    ) -> Self {
        Self {
            rx,
            game,
            player_handlers,
        }
    }

    async fn run(mut self) -> anyhow::Result<()> {
        while self.run_inner().await? == GameInstanceStatus::KeepAlive {}

        Ok(())
    }

    /// # Lifecycle
    /// 1. Ask `game` to generate a new `GameEvent`.
    /// 2. Send the `GameEvent` to each player and wait for all players to respond.
    /// 3. Provide the players' responses to `game`.
    /// 4. `game` verifies the responses.
    /// 5. `game` standbys (waiting for step 1 again).
    async fn run_inner(&mut self) -> anyhow::Result<GameInstanceStatus> {
        let event_for_each_player = match self.game.next_event() {
            Ok(v) => v,
            Err(e) => match e {
                NextEventError::EventProcessing => {
                    bail!("server internal error: unexpected game state");
                }
                NextEventError::NoMoreEvent => {
                    return Ok(GameInstanceStatus::ShouldShutdown);
                }
            },
        };

        for (player_id, game_ev) in event_for_each_player {
            debug!("new GameEvent for {:?}: {:?}", player_id, game_ev);

            self.player_handlers
                .get_mut(&player_id)
                .context(format!(
                    "server internal error: unknown player: {:?}",
                    player_id
                ))?
                .send_game_event(game_ev)?;
        }

        loop {
            let Some(ev) = self.rx.recv().await else {
                bail!("server internal error: channel closed");
            };

            match ev {
                ServerInternalEvent::RequestJoin(_) => {
                    warn!("invalid event: RequestJoin");
                }
                ServerInternalEvent::ConnectionLost(player_id) => {
                    if let Err(e) = self.verify_player_id(player_id) {
                        warn!("{}", e);
                        continue;
                    }

                    for (_, handler) in self
                        .player_handlers
                        .iter_mut()
                        .filter(|(id, _)| **id != player_id)
                    {
                        handler.notify_player_disconnected(player_id)?;
                    }
                }
                ServerInternalEvent::In(player_id, ev) => {
                    if let Err(e) = self.verify_player_id(player_id) {
                        warn!("{}", e);
                        continue;
                    }

                    let Some(game_event_resp) = self
                        .player_handlers
                        .get_mut(&player_id)
                        .expect("should be `Some`; the ID is verified")
                        .check_for_game_event_response(ev)
                    else {
                        continue;
                    };

                    match self.game.store_player_response(player_id, game_event_resp) {
                        Ok(true) => break,
                        Ok(false) => continue,
                        Err(e) => return Err(e),
                    }
                }
                unexpected => {
                    warn!("unexpected event: {:?}", unexpected);
                }
            }
        }

        Ok(GameInstanceStatus::KeepAlive)
    }

    fn verify_player_id(&self, player_id: PlayerId) -> anyhow::Result<()> {
        if !self.player_handlers.keys().any(|id| *id == player_id) {
            bail!("unknown PlayerId: {:?}", player_id);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameInstanceStatus {
    KeepAlive,
    ShouldShutdown,
}
