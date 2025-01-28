use crate::game::{CardInstance, CARD_DEPTH, GAME_SCOPE};
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
    fn spawn_cards<'a>(&'a mut self, commands: &'a mut Commands, at: Transform) -> Vec<Entity>;
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
    fn spawn_cards<'a>(&'a mut self, commands: &'a mut Commands, at: Transform) -> Vec<Entity> {
        let cards = (0..self.size).map(|_| self.get_messy_card());
        spawn_cards_impl(cards, commands, at).collect()
    }
}

pub struct SingleVariant {
    pub card: CardView,
    pub size: u8,
}

impl SpawnCards for SingleVariant {
    fn spawn_cards<'a>(&'a mut self, commands: &'a mut Commands, at: Transform) -> Vec<Entity> {
        let cards = std::iter::repeat_n(self.card, self.size as usize);
        spawn_cards_impl(cards, commands, at).collect()
    }
}

pub struct Fixed(pub Vec<CardView>);

impl SpawnCards for Fixed {
    fn spawn_cards<'a>(&'a mut self, commands: &'a mut Commands, at: Transform) -> Vec<Entity> {
        spawn_cards_impl(self.0.iter().rev().cloned(), commands, at).collect()
    }
}

pub struct Real;

impl SpawnCards for Real {
    fn spawn_cards<'a>(&'a mut self, commands: &'a mut Commands, at: Transform) -> Vec<Entity> {
        let mut cards = (0..=11)
            .into_iter()
            .cartesian_product([CardColor::Black, CardColor::White])
            .map(|(n, c)| CardView::from_props(c, Some(CardNumber(n)), false))
            .collect::<Vec<_>>();

        cards.shuffle(&mut rand::rng());

        spawn_cards_impl(cards, commands, at).collect()
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
            .spawn((GAME_SCOPE, CardInstance(card), transform))
            .id();

        id
    })
}
