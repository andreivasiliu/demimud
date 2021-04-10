//! Primitives to help with matching and filtering entities.
//!
//! These are very similar to standard iterators and streams, but they can also
//! store the reason why an object was filtered out, which can then be echoed
//! as an informative error message to the user.
//!
//! This module is black magic. I think that when GATs and coroutines arrive,
//! a lot of the phantoms and extra generic parameters can be replaced with
//! generic output types on traits.
//!
//! It implements a trait, `EntityIterator`, that allows for code like:
//!
//! ```ignore
//! let found = myself
//!     .contained_entities()  -> impl Iterator<Item = EntityInfo>
//!     .filter_by_keyword(target)  -> impl Iterator<Item = MatchCandidate>
//!     .filter_or(|e| e.is_mobile() || e.is_player(), "$^$N is not a creature!")  -> impl Iterator<Item = MatchCandidate>
//!     .prefer(|e| e.is_player())  -> impl Iterator<Item = MatchCandidate>
//!     .filter_or(|e| *e != myself, "You can't do that with yourself!")  -> impl Iterator<Item = MatchCandidate>
//!     .find_one_or("You don't see anything here!");  -> Result<EntityInfo, MatchError>
//!
//! let target = match found {
//!     Ok(target) => target,
//!     Err(error) => return self.echo_error(error),
//! };
//! ```
//! 
//! The `EntityIterator` is implemented for both iterators of entities, as
//! well as iterators of match candidates.
//!
//! A `MatchCandidate` is similar to a Result, but instead of removing entities
//! that don't match, the candidate is changed to contain the reason why.
//!
//! If no match is found, a `MatchCandidate` that matched the most filters
//! before being turned into an error is used to get an error message.

use std::marker::PhantomData;

use crate::{components::ComponentFromEntity, entity::{EntityId, EntityInfo}};

pub(crate) enum MatchError {
    /// An error message where $N will be replaced with the current entity
    Message(&'static str),

    /// An error message where $N will be replaced with a user-given entity
    MessageWithActor(&'static str, EntityId),
}

pub(crate) enum MatchCandidate<'e, Component> {
    GoodMatch {
        entity: EntityInfo<'e>,
        matched_conditions: u8,
        preferred: Option<bool>,
        component: &'e Component,
    },
    BadMatch {
        entity: EntityInfo<'e>,
        matched_conditions: u8,
        error: MatchError,
    },
}

use MatchCandidate::{BadMatch, GoodMatch};

pub(crate) trait IntoMatchCandidate<'e> {
    type Component;

    fn into_match_candidate(self) -> MatchCandidate<'e, Self::Component>;
}

impl<'e> IntoMatchCandidate<'e> for EntityInfo<'e> {
    type Component = ();

    fn into_match_candidate(self) -> MatchCandidate<'e, ()> {
        MatchCandidate::GoodMatch {
            entity: self,
            component: &(),
            matched_conditions: 0,
            preferred: None,
        }
    }
}

impl<'e, T> IntoMatchCandidate<'e> for MatchCandidate<'e, T> {
    type Component = T;

    fn into_match_candidate(self) -> MatchCandidate<'e, T> {
        self
    }
}

pub(crate) struct FilterByKeyword<'k, I> {
    inner: I,
    keyword: &'k str,
}

impl<'q, I> Iterator for FilterByKeyword<'q, I>
where
    I: EntityIterator<'q, ()>,
{
    type Item = MatchCandidate<'q, ()>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_match_candidate().filter(|item| {
            let entity = match item {
                GoodMatch { entity, .. } => entity,
                BadMatch { entity, .. } => entity,
            };
            entity
                .component_info()
                .keyword()
                .split_whitespace()
                .any(|word| word.eq_ignore_ascii_case(self.keyword))
        })
    }
}

pub(crate) struct FilterOrError<'q, I: 'q, P, C> {
    inner: I,
    predicate: P,
    error: &'static str,
    shadow: PhantomData<&'q C>,
}

impl<'q, I, P, C> Iterator for FilterOrError<'q, I, P, C>
where
    I: EntityIterator<'q, C>,
    P: Fn(&EntityInfo<'q>) -> bool,
{
    type Item = MatchCandidate<'q, C>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_match_candidate().map(|item| match item {
            GoodMatch {
                entity,
                matched_conditions,
                preferred,
                component,
            } => {
                if (self.predicate)(&entity) {
                    GoodMatch {
                        entity,
                        matched_conditions: matched_conditions + 1,
                        preferred,
                        component,
                    }
                } else {
                    BadMatch {
                        matched_conditions,
                        error: MatchError::MessageWithActor(self.error, entity.entity_id()),
                        entity,
                    }
                }
            }
            item @ BadMatch { .. } => item,
        })
    }
}

pub(crate) struct Prefer<'q, I: 'q, P> {
    inner: I,
    predicate: P,
    shadow: PhantomData<&'q ()>,
}

impl<'q, I, P> Iterator for Prefer<'q, I, P>
where
    I: EntityIterator<'q, ()>,
    P: Fn(&EntityInfo<'q>) -> bool,
{
    type Item = MatchCandidate<'q, ()>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_match_candidate().map(|mut item| {
            match &mut item {
                GoodMatch {
                    entity, preferred, ..
                } => {
                    let new_preferred = (self.predicate)(&entity);
                    *preferred = Some(new_preferred);
                }
                BadMatch { .. } => (),
            }
            item
        })
    }
}

pub(crate) struct PreferComponent<'q, I: 'q, P, C> {
    inner: I,
    predicate: P,
    shadow: PhantomData<&'q C>,
}

impl<'q, I, P, C> Iterator for PreferComponent<'q, I, P, C>
where
    I: EntityIterator<'q, C>,
    P: Fn(&EntityInfo<'q>, &C) -> bool,
{
    type Item = MatchCandidate<'q, C>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_match_candidate().map(|mut item| {
            match &mut item {
                GoodMatch {
                    entity, preferred, component, ..
                } => {
                    let new_preferred = (self.predicate)(&entity, component);
                    *preferred = Some(new_preferred);
                }
                BadMatch { .. } => (),
            }
            item
        })
    }
}

pub(crate) struct WithComponentOrError<'q, I: 'q, C1, C2: 'q> {
    inner: I,
    error: &'static str,
    shadow1: PhantomData<&'q C1>,
    shadow2: PhantomData<&'q C2>,
}

impl<'q, I, C1, C2: 'q> Iterator for WithComponentOrError<'q, I, C1, C2>
where
    I: EntityIterator<'q, C1>,
    C2: ComponentFromEntity,
{
    type Item = MatchCandidate<'q, C2>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next_match_candidate().map(|item| match item {
            GoodMatch {
                entity,
                matched_conditions,
                preferred,
                component: _,
            } => {
                if let Some(component) = C2::component_from_entity(&entity) {
                    GoodMatch {
                        entity,
                        component,
                        matched_conditions: matched_conditions + 1,
                        preferred,
                    }
                } else {
                    BadMatch {
                        matched_conditions,
                        error: MatchError::MessageWithActor(self.error, entity.entity_id()),
                        entity,
                    }
                }
            }
            // Change from MatchCandidate<C1> to MatchCandidate<C2>
            BadMatch {
                entity,
                matched_conditions,
                error,
            } => BadMatch {
                entity,
                matched_conditions,
                error,
            },
        })
    }
}

pub(crate) struct WithComponent<'q, I: 'q, C1, C2: 'q> {
    inner: I,
    shadow1: PhantomData<&'q C1>,
    shadow2: PhantomData<&'q C2>,
}

impl<'q, I, C1, C2: 'q> Iterator for WithComponent<'q, I, C1, C2>
where
    I: EntityIterator<'q, C1>,
    C2: ComponentFromEntity,
{
    type Item = MatchCandidate<'q, C2>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let item = self.inner.next_match_candidate();
            
            return if let Some(item) = item {
                Some(match item {
                    GoodMatch {
                        entity,
                        matched_conditions,
                        preferred,
                        component: _,
                    } => {
                        if let Some(component) = C2::component_from_entity(&entity) {
                            GoodMatch {
                                entity,
                                component,
                                matched_conditions: matched_conditions + 1,
                                preferred,
                            }
                        } else {
                            continue;
                        }
                    }
                    // Change from MatchCandidate<C1> to MatchCandidate<C2>
                    // Also filter errors that don't have the right component
                    BadMatch {
                        entity,
                        matched_conditions,
                        error,
                    } => {
                        if C2::component_from_entity(&entity).is_some() {
                            BadMatch {
                                entity,
                                matched_conditions,
                                error,
                            }
                        } else {
                            continue;
                        }
                    },
                })
            } else {
                None
            }
        }
    }
}

pub(crate) trait EntityIterator<'e, C: 'e>: Sized {
    fn next_match_candidate(&mut self) -> Option<MatchCandidate<'e, C>>;

    fn prefer<'p, P: Fn(&EntityInfo<'p>) -> bool>(self, prefer: P) -> Prefer<'p, Self, P> {
        Prefer {
            inner: self,
            predicate: prefer,
            shadow: PhantomData::default(),
        }
    }

    fn prefer_component<'p, P: Fn(&EntityInfo<'p>, &C) -> bool>(self, prefer: P) -> PreferComponent<'p, Self, P, C> {
        PreferComponent {
            inner: self,
            predicate: prefer,
            shadow: PhantomData::default(),
        }
    }

    fn with_component_or<NewComponent>(
        self,
        error: &'static str,
    ) -> WithComponentOrError<'e, Self, C, NewComponent> {
        WithComponentOrError {
            inner: self,
            error,
            shadow1: PhantomData::default(),
            shadow2: PhantomData::default(),
        }
    }

    fn with_component<NewComponent>(
        self,
    ) -> WithComponent<'e, Self, C, NewComponent> {
        WithComponent {
            inner: self,
            shadow1: PhantomData::default(),
            shadow2: PhantomData::default(),
        }
    }

    fn filter_by_keyword(self, keyword: &str) -> FilterByKeyword<'_, Self> {
        FilterByKeyword {
            inner: self,
            keyword,
        }
    }

    fn filter_or<'p, F: Fn(&EntityInfo<'p>) -> bool>(
        self,
        filter: F,
        error: &'static str,
    ) -> FilterOrError<'p, Self, F, C> {
        FilterOrError {
            inner: self,
            predicate: filter,
            error,
            shadow: PhantomData::default(),
        }
    }

    fn find_one_with_component_or(mut self, error: &'static str) -> Result<(EntityInfo<'e>, &'e C), MatchError> {
        let mut bad_match = None;
        let mut bad_match_conditions = 0;
        let mut unpreferred_match = None;

        while let Some(item) = self.next_match_candidate() {
            match item {
                GoodMatch {
                    entity,
                    preferred: Some(false),
                    component,
                    ..
                } => {
                    unpreferred_match = Some((entity, component));
                }
                GoodMatch { entity, component, .. } => {
                    return Ok((entity, component));
                }
                BadMatch {
                    matched_conditions,
                    error,
                    ..
                } => {
                    if matched_conditions >= bad_match_conditions {
                        bad_match = Some(error);
                        bad_match_conditions = matched_conditions;
                    }
                }
            }
        }

        if let Some(unpreferred_match) = unpreferred_match {
            Ok(unpreferred_match)
        } else if let Some(bad_match) = bad_match {
            Err(bad_match)
        } else {
            Err(MatchError::Message(error))
        }
    }

    fn find_one_or(self, error: &'static str) -> Result<EntityInfo<'e>, MatchError> {
        self.find_one_with_component_or(error).map(|result| result.0)
    }
}

impl<'e, I, T> EntityIterator<'e, T::Component> for I
where
    T: IntoMatchCandidate<'e> + 'e,
    I: Iterator<Item = T>,
{
    fn next_match_candidate(&mut self) -> Option<MatchCandidate<'e, T::Component>> {
        self.next().map(|e| e.into_match_candidate())
    }
}
