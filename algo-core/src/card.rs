use anyhow::{bail, Context as _};
use itertools::Itertools as _;
use rand::seq::SliceRandom as _;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// Possible card colors
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum CardColor {
    Black,
    White,
}

impl CardColor {
    /// Returns a background color in RGB format.
    pub fn bg_color_rgb(&self) -> [u8; 3] {
        match self {
            Self::Black => [0; 3],
            Self::White => [u8::MAX; 3],
        }
    }

    /// Returns a text color in RGB format.
    pub fn text_color_rgb(&self) -> [u8; 3] {
        match self {
            Self::Black => [u8::MAX; 3],
            Self::White => [0; 3],
        }
    }

    fn symbol(&self) -> u8 {
        match self {
            CardColor::Black => b'B',
            CardColor::White => b'W',
        }
    }
}

impl FromStr for CardColor {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let color = match &*s.trim().to_lowercase() {
            "black" => Self::Black,
            "white" => Self::White,
            unknown => bail!("unknown CardColor: {}", unknown),
        };

        Ok(color)
    }
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

impl PartialEq<CardNumberType> for CardNumber {
    fn eq(&self, other: &CardNumberType) -> bool {
        self.0.eq(other)
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
        Some(self.cmp(other))
    }
}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Manually specifying the priority just in case.
        match self.priv_info.number.cmp(&other.priv_info.number) {
            std::cmp::Ordering::Equal => self.pub_info.color.cmp(&other.pub_info.color),
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

    pub fn full_view(self) -> CardView {
        CardView {
            pub_info: self.pub_info,
            priv_info: Some(self.priv_info),
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

// TODO: Refactor the struct into an enum to eliminate an invalid state
//       where a card is revealed but its number is missing.
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
        let revealed = self.pub_info.revealed;
        write!(
            f,
            "{:?}-{}{}{}",
            self.pub_info.color,
            if revealed { "" } else { "(" },
            self.priv_info
                .map_or("?".into(), |v| v.number.0.to_string()),
            if revealed { "" } else { ")" },
        )
    }
}

impl FromStr for CardView {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split('-');
        let color = iter.next().context("CardColor is missing")?.parse()?;

        let ret = match iter.next().context("CardNumber is missing")?.trim() {
            "?" => Self {
                pub_info: CardPubInfo {
                    color,
                    revealed: false,
                },
                priv_info: None,
            },
            n if n.starts_with('(') && n.ends_with(')') => Self {
                pub_info: CardPubInfo {
                    color,
                    revealed: false,
                },
                priv_info: Some(CardPrivInfo::new(CardNumber(n[1..n.len() - 1].parse()?))),
            },
            n => Self {
                pub_info: CardPubInfo {
                    color,
                    revealed: true,
                },
                priv_info: Some(CardPrivInfo::new(CardNumber(n.parse()?))),
            },
        };

        Ok(ret)
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
            cards: self.cards.iter().map(|v| v.pub_info.color).collect(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TalonView {
    pub cards: Vec<CardColor>,
}

impl fmt::Debug for TalonView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cards = self.cards.iter().map(|v| v.symbol()).collect::<Vec<_>>();
        f.debug_tuple("TalonView")
            .field(&unsafe { String::from_utf8_unchecked(cards) })
            .finish()
        // SAFETY: CardColor::symbol returns only valid UTF-8.
    }
}
