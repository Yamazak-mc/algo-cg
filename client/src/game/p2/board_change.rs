use client::utils::observer_controller;

use super::*;

pub fn board_change_plugin(app: &mut App) {
    app.add_state_scoped_observer_named(P2_CTX_STATE, ApplyBoardChange::talon_to_field)
        .add_state_scoped_observer_named(P2_CTX_STATE, ApplyBoardChange::talon_to_attacker)
        .add_state_scoped_observer_named(P2_CTX_STATE, ApplyBoardChange::attacker_to_field)
        .add_state_scoped_observer_named(P2_CTX_STATE, ApplyBoardChange::reveal_attacker)
        .add_state_scoped_observer_named(P2_CTX_STATE, ApplyBoardChange::reveal_field_card);
}

#[derive(Deref, DerefMut, Event)]
pub struct ApplyBoardChange(pub BoardChange);

impl ApplyBoardChange {
    fn talon_to_field(
        trigger: Trigger<Self>,
        mut talon_top_idx: Single<&mut TalonTopCardIndex>,
        talon_cards: Query<(Entity, &TalonCardIndex)>,
        mut commands: Commands,
        mut fields: Query<(Entity, &CardFieldOwnedBy, &mut CardField)>,
    ) {
        let BoardChange::CardMoved {
            player,
            movement: CardMovement::TalonToField { insert_at },
            card,
        } = trigger.event().0
        else {
            return;
        };

        // Update talon
        talon_top_idx.0 -= 1;
        let (card_entity, _) = talon_cards
            .iter()
            .find(|(_, idx)| idx.0 == talon_top_idx.0)
            .unwrap();

        // If private info is provided, attach that to the card
        if let Some(priv_info) = card.priv_info {
            commands.trigger_targets(card_instance::AddPrivInfo(priv_info), card_entity);
        }

        // Insert card to the owner's field
        let (field_entity, _, mut field) = fields
            .iter_mut()
            .find(|(_, owned_by, _)| owned_by.0 == player)
            .unwrap();
        field.insert_card(field_entity, insert_at, card_entity, &mut commands);

        commands.trigger_targets(insert_observer_controller(), card_entity);
    }

    fn talon_to_attacker(
        trigger: Trigger<Self>,
        mut talon_top_idx: Single<&mut TalonTopCardIndex>,
        talon_cards: Query<(Entity, &TalonCardIndex)>,
        mut commands: Commands,
    ) {
        let BoardChange::CardMoved {
            player: _,
            movement: CardMovement::TalonToAttacker,
            card,
        } = trigger.event().0
        else {
            return;
        };

        // Update talon
        talon_top_idx.0 -= 1;
        let (card_entity, _) = talon_cards
            .iter()
            .find(|(_, idx)| idx.0 == talon_top_idx.0)
            .unwrap();

        // If private info is provided, attach that to the card
        if let Some(priv_info) = card.priv_info {
            commands.trigger_targets(card_instance::AddPrivInfo(priv_info), card_entity);
        }

        commands
            .entity(card_entity)
            .insert(Attacker)
            .trigger(AnimateTransform::new(
                ATTACKER_XF,
                0.5,
                EaseFunction::QuarticOut,
            ));

        commands.trigger_targets(insert_observer_controller(), card_entity);
    }

    fn attacker_to_field(
        trigger: Trigger<Self>,
        attacker: Option<Single<Entity, With<Attacker>>>,
        mut commands: Commands,
        mut fields: Query<(Entity, &CardFieldOwnedBy, &mut CardField)>,
    ) {
        let BoardChange::CardMoved {
            player,
            movement: CardMovement::AttackerToField { insert_at },
            card: _,
        } = trigger.event().0
        else {
            return;
        };

        // Insert card to the owner's field
        let (field_entity, _, mut field) = fields
            .iter_mut()
            .find(|(_, owned_by, _)| owned_by.0 == player)
            .unwrap();
        let attacker = *attacker.unwrap();
        field.insert_card(field_entity, insert_at, attacker, &mut commands);
        commands.entity(attacker).remove::<Attacker>();
    }

    fn reveal_attacker(
        trigger: Trigger<Self>,
        attacker: Option<Single<(Entity, &CardInstance), With<Attacker>>>,
        mut commands: Commands,
    ) {
        let BoardChange::CardRevealed {
            player: _,
            location: CardLocation::Attacker,
            card,
        } = trigger.event().0
        else {
            return;
        };

        let (card_entity, card_inst) = *attacker.unwrap();

        if card_inst.get().priv_info.is_some() {
            commands.trigger_targets(card_instance::Reveal, card_entity);
        } else {
            commands.trigger_targets(card_instance::RevealWith(card.priv_info), card_entity);
        }
    }

    fn reveal_field_card(
        trigger: Trigger<Self>,
        mut fields: Query<(&CardFieldOwnedBy, &mut CardField)>,
        cards: Query<&CardInstance>,
        mut commands: Commands,
    ) {
        let BoardChange::CardRevealed {
            player,
            location: CardLocation::Field { idx },
            card,
        } = trigger.event().0
        else {
            return;
        };

        let (_, field) = fields
            .iter_mut()
            .find(|(owned_by, _)| owned_by.0 == player)
            .unwrap();

        let card_entity = field.cards()[idx as usize];
        let card_inst = cards.get(card_entity).unwrap();

        if card_inst.get().priv_info.is_some() {
            commands.trigger_targets(card_instance::Reveal, card_entity);
        } else {
            commands.trigger_targets(card_instance::RevealWith(card.priv_info), card_entity);
        }
    }
}

fn insert_observer_controller() -> impl Event {
    observer_controller::Insert::<Pointer<Click>>::new_paused(|| {
        Observer::new(super::on_click_attack_target)
    })
}
