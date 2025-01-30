use crate::{
    game::{card::instance::CardInstance, CARD_DEPTH},
    AppState,
};
use algo_core::card::{CardColor, CardNumber, CardPrivInfo, CardPubInfo, CardView};
use bevy::prelude::*;
use itertools::Itertools as _;
use rand::{
    rngs::ThreadRng,
    seq::{IndexedRandom as _, SliceRandom as _},
    Rng as _,
};

pub struct SandboxTalon {
    spawner: Box<dyn SpawnCards + 'static>,
    cards: Vec<Entity>,
}

impl SandboxTalon {
    pub fn new(spawner: impl SpawnCards + 'static) -> Self {
        Self {
            spawner: Box::new(spawner),
            cards: Vec::new(),
        }
    }

    /// Initializes the talon cards.
    ///
    /// The reusability of this function is undefined.
    pub fn init(&mut self, commands: &mut Commands, at: Transform) {
        self.cards = self.spawner.spawn_cards(commands, at);
    }

    pub fn draw_card(&mut self) -> Option<Entity> {
        self.cards.pop()
    }

    pub fn peek_card(&mut self) -> Option<Entity> {
        self.cards.last().cloned()
    }
}

pub trait SpawnCards {
    fn produce_cards(&mut self) -> Vec<CardView>;

    fn spawn_cards<'a>(&'a mut self, commands: &'a mut Commands, at: Transform) -> Vec<Entity> {
        spawn_cards_impl(self.produce_cards(), commands, at).collect()
    }
}

impl SpawnCards for Vec<CardView> {
    fn produce_cards(&mut self) -> Vec<CardView> {
        self.iter().cloned().rev().collect()
    }
}

pub struct Messy {
    size: u8,
    rng: ThreadRng,
}

impl Messy {
    #[allow(unused)]
    pub fn new(size: u8) -> Self {
        Self {
            size,
            rng: rand::rng(),
        }
    }

    fn get_messy_card(&mut self) -> CardView {
        let rng = &mut self.rng;

        let color = *[CardColor::White, CardColor::Black].choose(rng).unwrap();
        let number = rng.random_range(0..=87);
        let revealed = rng.random();

        CardView {
            pub_info: CardPubInfo { color, revealed },
            priv_info: revealed.then_some(CardPrivInfo {
                number: number.into(),
            }),
        }
    }
}

impl SpawnCards for Messy {
    fn produce_cards(&mut self) -> Vec<CardView> {
        (0..self.size).map(|_| self.get_messy_card()).collect()
    }
}

pub struct SingleVariant {
    pub card: CardView,
    pub size: u8,
}

impl SpawnCards for SingleVariant {
    fn produce_cards(&mut self) -> Vec<CardView> {
        std::iter::repeat_n(self.card, self.size as usize).collect()
    }
}

pub struct Real;

impl SpawnCards for Real {
    fn produce_cards(&mut self) -> Vec<CardView> {
        let mut cards = (0..=11)
            .cartesian_product([CardColor::Black, CardColor::White])
            .map(|(n, c)| CardView::from_props(c, Some(CardNumber(n)), false))
            .collect::<Vec<_>>();

        cards.shuffle(&mut rand::rng());

        cards
    }
}

pub struct Map<S, F> {
    spawner: S,
    map_fn: F,
}

impl<S, F> Map<S, F>
where
    S: SpawnCards + 'static,
    F: FnMut((usize, CardView)) -> CardView,
{
    #[allow(unused)]
    pub fn new(spawner: S, map_fn: F) -> Self {
        Self { spawner, map_fn }
    }
}

impl<S, F> SpawnCards for Map<S, F>
where
    S: SpawnCards + 'static,
    F: FnMut((usize, CardView)) -> CardView,
{
    fn produce_cards(&mut self) -> Vec<CardView> {
        self.spawner
            .produce_cards()
            .into_iter()
            .rev()
            .enumerate()
            .map(&mut self.map_fn)
            .rev()
            .collect()
    }
}

fn spawn_cards_impl<'a, 'w, 's, T>(
    cards: T,
    commands: &'a mut Commands<'w, 's>,
    at: Transform,
) -> impl Iterator<Item = Entity> + use<'a, 'w, 's, T>
where
    T: IntoIterator<Item = CardView>,
{
    cards.into_iter().enumerate().map(move |(i, card)| {
        let mut transform = at;
        transform.translation.y += i as f32 * CARD_DEPTH;

        let id = commands
            .spawn((
                StateScoped(AppState::Game),
                CardInstance::new(card),
                transform,
            ))
            .id();

        id
    })
}
