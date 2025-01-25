use itertools::Itertools as _;
use rand::seq::SliceRandom as _;
use serde::{Deserialize, Serialize};

/// Possible card colors
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum CardColor {
    Black,
    White,
}

pub type CardNumberType = u8;

/// A number assigned to the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct CardNumber(pub CardNumberType);

impl From<CardNumberType> for CardNumber {
    fn from(value: CardNumberType) -> Self {
        Self(value)
    }
}

/// A main card structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Card {
    pub pub_info: CardPubInfo,
    pub priv_info: CardPrivInfo,
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Manually specifying the priority just in case.
        match self.priv_info.number.partial_cmp(&other.priv_info.number) {
            Some(std::cmp::Ordering::Equal) => {
                self.pub_info.color.partial_cmp(&other.pub_info.color)
            }
            v => v,
        }
    }
}

impl Card {
    pub fn new(number: CardNumber, color: CardColor) -> Self {
        Self {
            pub_info: CardPubInfo::new(color),
            priv_info: CardPrivInfo::new(number),
        }
    }

    /// Returns the information of the card that is visible to the public.
    pub fn public_view(&self) -> CardView {
        if self.pub_info.revealed() {
            CardView::full(*self)
        } else {
            CardView::hidden(*self)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CardPubInfo {
    pub color: CardColor,
    pub revealed: bool,
}

impl CardPubInfo {
    pub fn new(color: CardColor) -> Self {
        Self {
            color,
            revealed: false,
        }
    }

    pub fn color(&self) -> CardColor {
        self.color
    }

    pub fn revealed(&self) -> bool {
        self.revealed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CardPrivInfo {
    pub number: CardNumber,
}

impl CardPrivInfo {
    pub fn new(number: CardNumber) -> Self {
        Self { number }
    }

    pub fn number(&self) -> CardNumber {
        self.number
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CardView {
    pub pub_info: CardPubInfo,
    pub priv_info: Option<CardPrivInfo>,
}

impl CardView {
    /// Constructs a new `CardView`.
    ///
    /// NOTE: This function can create an invalid state where
    ///       the card is revealed but the number is missing.
    #[allow(unused)]
    pub fn from_props(color: CardColor, number: Option<CardNumber>, revealed: bool) -> Self {
        let pub_info = CardPubInfo { color, revealed };
        let priv_info = number.map(CardPrivInfo::new);

        Self {
            pub_info,
            priv_info,
        }
    }

    fn full(card: Card) -> Self {
        Self {
            pub_info: card.pub_info,
            priv_info: Some(card.priv_info),
        }
    }

    fn hidden(card: Card) -> Self {
        Self {
            pub_info: card.pub_info,
            priv_info: None,
        }
    }

    pub(crate) fn public_view(&self) -> Self {
        if self.pub_info.revealed() {
            *self
        } else {
            Self {
                priv_info: None,
                ..*self
            }
        }
    }
}

impl std::fmt::Display for CardView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}-{}",
            self.pub_info.color,
            self.priv_info
                .map_or("?".into(), |v| v.number.0.to_string())
        )
    }
}

/// Returns a list of card instances to add to the talon.
pub(crate) fn create_cards<J>(
    numbers: impl IntoIterator<Item = CardNumberType>,
    colors: J,
) -> impl Iterator<Item = Card>
where
    J: IntoIterator<Item = CardColor>,
    J::IntoIter: Clone,
{
    numbers
        .into_iter()
        .cartesian_product(colors)
        .map(|(n, c)| Card::new(n.into(), c))
}

/// A stack of cards that players can draw from during the game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Talon {
    cards: Vec<Card>,
}

impl FromIterator<Card> for Talon {
    fn from_iter<T: IntoIterator<Item = Card>>(iter: T) -> Self {
        let cards = iter.into_iter().collect();
        Self { cards }
    }
}

impl Talon {
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn view_top(&self) -> Option<CardView> {
        self.cards.last().map(|v| v.public_view())
    }

    pub fn shuffle(&mut self, mut rng: impl rand::Rng) {
        self.cards.shuffle(&mut rng);
    }

    pub fn draw(&mut self) -> Option<Card> {
        self.cards.pop()
    }

    pub fn view(&self) -> TalonView {
        TalonView {
            top_card: self.cards.last().map(|v| v.pub_info),
            cards_remaining: self.cards.len() as u32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct TalonView {
    pub top_card: Option<CardPubInfo>,
    pub cards_remaining: u32,
}
