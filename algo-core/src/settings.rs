use crate::card::{create_cards, Card, CardColor, CardNumberType};
use anyhow::bail;

const MAX_CARD_NUM_DEFAULT: CardNumberType = 11;
const COLOR_VARIANTS_MIN: usize = 2;
const INITIAL_DRAW_NUM: u32 = 4;

/// Game settings
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSettings {
    /// Color variants to include.
    pub card_colors: Vec<CardColor>,

    /// A maximum card number.
    pub max_card_number: CardNumberType,

    /// A number of cards for each player to draw when the game is started.
    pub initial_draw_num: u32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            card_colors: vec![CardColor::Black, CardColor::White],
            max_card_number: MAX_CARD_NUM_DEFAULT,
            initial_draw_num: INITIAL_DRAW_NUM,
        }
    }
}

impl GameSettings {
    pub(crate) fn build_cards(self) -> anyhow::Result<Vec<Card>> {
        if self.card_colors.len() < COLOR_VARIANTS_MIN {
            bail!("there must be at least {} card colors", COLOR_VARIANTS_MIN);
        }

        if self.max_card_number < MAX_CARD_NUM_DEFAULT {
            bail!(
                "max_card_number must be greater than {}",
                MAX_CARD_NUM_DEFAULT
            );
        }

        let ret = create_cards(0..=self.max_card_number, self.card_colors).collect();
        Ok(ret)
    }
}
