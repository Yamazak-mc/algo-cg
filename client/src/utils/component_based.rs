use bevy::prelude::*;

pub trait EnableComponentBased {
    fn enable_component_based<T: Component, Src: Component>(&mut self) -> &mut Self;
}

impl EnableComponentBased for App {
    fn enable_component_based<T: Component, Src: Component>(&mut self) -> &mut Self {
        self.add_systems(Update, component_based_system::<T, Src>)
    }
}

pub fn component_based_system<T: Component, Src: Component>(
    mut query: Query<(&mut T, &Src, &ComponentBased<T, Src>), Changed<Src>>,
) {
    for (mut val, src, map_fn) in &mut query {
        // debug!(
        //     "component_based<{}, {}>",
        //     std::any::type_name::<T>(),
        //     std::any::type_name::<Src>()
        // );
        *val = (map_fn.0)(src);
    }
}

#[derive(Component)]
pub struct ComponentBased<T, Src>(Box<dyn Fn(&Src) -> T + Send + Sync + 'static>);

impl<T, Src> ComponentBased<T, Src> {
    pub fn new(map_fn: impl Fn(&Src) -> T + Send + Sync + 'static) -> Self {
        Self(Box::new(map_fn))
    }
}

pub fn interaction_based<T: Component + Clone>(
    pressed: T,
    hovered: T,
    none: T,
) -> ComponentBased<T, Interaction> {
    ComponentBased::new(move |src| match src {
        Interaction::Pressed => pressed.clone(),
        Interaction::Hovered => hovered.clone(),
        Interaction::None => none.clone(),
    })
}

pub trait EnableParentComponentBased {
    fn enable_parent_component_based<T: Component, Src: Component>(&mut self) -> &mut Self;
}

impl EnableParentComponentBased for App {
    fn enable_parent_component_based<T: Component, Src: Component>(&mut self) -> &mut Self {
        self.add_systems(Update, parent_component_based_system::<T, Src>)
    }
}

pub fn parent_component_based_system<T: Component, Src: Component>(
    query: Query<(&Src, &Children), Changed<Src>>,
    mut children_query: Query<(&mut T, &ParentComponentBased<T, Src>)>,
) {
    for (src, children) in &query {
        for child in children {
            if let Ok((mut data, map_fn)) = children_query.get_mut(*child) {
                *data = (map_fn.0)(src);
            }
        }
    }
}

#[derive(Component)]
pub struct ParentComponentBased<T, Src>(Box<dyn Fn(&Src) -> T + Send + Sync + 'static>);

impl<T, Src> ParentComponentBased<T, Src> {
    pub fn new(map_fn: impl Fn(&Src) -> T + Send + Sync + 'static) -> Self {
        Self(Box::new(map_fn))
    }
}

pub fn parent_interaction_based<T: Component + Clone>(
    pressed: T,
    hovered: T,
    none: T,
) -> ParentComponentBased<T, Interaction> {
    ParentComponentBased::new(move |src| match src {
        Interaction::Pressed => pressed.clone(),
        Interaction::Hovered => hovered.clone(),
        Interaction::None => none.clone(),
    })
}
