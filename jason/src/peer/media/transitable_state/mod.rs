//! [`Disableable`]s media exchange state.
//!
//! [`Disableable`]: super::Disableable

mod controller;
mod media_exchange;
mod mute;

pub use self::{
    controller::{
        MediaExchangeStateController, MuteStateController,
        TransitableStateController,
    },
    media_exchange::{StableMediaExchangeState, TransitionMediaExchangeState},
    mute::{StableMuteState, TransitionMuteState},
};

pub type MediaExchangeState =
    TransitableState<StableMediaExchangeState, TransitionMediaExchangeState>;
pub type MuteState = TransitableState<StableMuteState, TransitionMuteState>;

/// All media exchange states in which [`Disableable`] can be.
///
/// [`Disableable`]: super::Disableable
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitableState<S, T> {
    /// State of transition.
    Transition(T),

    /// Stable state.
    Stable(S),
}

impl From<StableMediaExchangeState> for MediaExchangeState {
    fn from(from: StableMediaExchangeState) -> Self {
        Self::Stable(from)
    }
}

impl From<TransitionMediaExchangeState> for MediaExchangeState {
    fn from(from: TransitionMediaExchangeState) -> Self {
        Self::Transition(from)
    }
}

impl From<StableMuteState> for MuteState {
    fn from(from: StableMuteState) -> Self {
        Self::Stable(from)
    }
}

impl From<TransitionMuteState> for MuteState {
    fn from(from: TransitionMuteState) -> Self {
        Self::Transition(from)
    }
}

impl<S, T> TransitableState<S, T>
where
    T: InTransition<Stable = S> + Into<TransitableState<S, T>>,
    S: InStable<Transition = T> + Into<TransitableState<S, T>>,
{
    /// Indicates whether [`MediaExchangeState`] is stable (not in transition).
    #[inline]
    pub fn is_stable(self) -> bool {
        match self {
            TransitableState::Stable(_) => true,
            TransitableState::Transition(_) => false,
        }
    }

    /// Starts transition into the `desired_state` changing the state to
    /// [`MediaExchangeState::Transition`].
    ///
    /// No-op if already in the `desired_state`.
    pub fn transition_to(self, desired_state: S) -> Self {
        if self == desired_state.into() {
            return self;
        }
        match self {
            Self::Stable(stable) => stable.start_transition().into(),
            Self::Transition(transition) => {
                if transition.intended() == desired_state {
                    self
                } else {
                    transition.reverse().into()
                }
            }
        }
    }

    /// Cancels ongoing transition if any.
    #[inline]
    pub fn cancel_transition(self) -> Self {
        match self {
            Self::Stable(_) => self,
            Self::Transition(t) => t.into_inner().into(),
        }
    }
}

pub trait InStable: Clone + Copy + PartialEq {
    type Transition: InTransition;

    fn start_transition(self) -> Self::Transition;
}

pub trait InTransition: Clone + Copy + PartialEq {
    type Stable: InStable;

    /// Returns intention which this [`MediaExchangeStateTransition`] indicates.
    fn intended(self) -> Self::Stable;

    /// Sets inner [`StableMediaExchangeState`].
    fn set_inner(self, inner: Self::Stable) -> Self;

    /// Returns inner [`StableMediaExchangeState`].
    fn into_inner(self) -> Self::Stable;

    fn reverse(self) -> Self;
}

#[cfg(test)]
mod test {
    use super::*;

    const DISABLED: TransitableState =
        TransitableState::Stable(StableMediaExchangeState::Disabled);
    const ENABLED: TransitableState =
        TransitableState::Stable(StableMediaExchangeState::Enabled);
    const ENABLING_DISABLED: TransitableState =
        TransitableState::Transition(TransitionMediaExchangeState::Enabling(
            StableMediaExchangeState::Disabled,
        ));
    const ENABLING_ENABLED: TransitableState =
        TransitableState::Transition(TransitionMediaExchangeState::Enabling(
            StableMediaExchangeState::Enabled,
        ));
    const DISABLING_DISABLED: TransitableState =
        TransitableState::Transition(TransitionMediaExchangeState::Disabling(
            StableMediaExchangeState::Disabled,
        ));
    const DISABLING_ENABLED: TransitableState =
        TransitableState::Transition(TransitionMediaExchangeState::Disabling(
            StableMediaExchangeState::Enabled,
        ));

    #[test]
    fn transition_to() {
        assert_eq!(
            DISABLED.transition_to(StableMediaExchangeState::Disabled),
            DISABLED
        );
        assert_eq!(
            DISABLED.transition_to(StableMediaExchangeState::Enabled),
            ENABLING_DISABLED
        );
        assert_eq!(
            ENABLED.transition_to(StableMediaExchangeState::Enabled),
            ENABLED
        );
        assert_eq!(
            ENABLED.transition_to(StableMediaExchangeState::Disabled),
            DISABLING_ENABLED
        );

        assert_eq!(
            ENABLING_DISABLED.transition_to(StableMediaExchangeState::Disabled),
            DISABLING_DISABLED
        );
        assert_eq!(
            ENABLING_DISABLED.transition_to(StableMediaExchangeState::Enabled),
            ENABLING_DISABLED
        );
        assert_eq!(
            DISABLING_ENABLED.transition_to(StableMediaExchangeState::Disabled),
            DISABLING_ENABLED
        );
        assert_eq!(
            DISABLING_ENABLED.transition_to(StableMediaExchangeState::Enabled),
            ENABLING_ENABLED
        );
        assert_eq!(
            DISABLING_DISABLED
                .transition_to(StableMediaExchangeState::Disabled),
            DISABLING_DISABLED
        );
        assert_eq!(
            DISABLING_DISABLED.transition_to(StableMediaExchangeState::Enabled),
            ENABLING_DISABLED
        );
        assert_eq!(
            ENABLING_ENABLED.transition_to(StableMediaExchangeState::Disabled),
            DISABLING_ENABLED
        );
        assert_eq!(
            ENABLING_ENABLED.transition_to(StableMediaExchangeState::Enabled),
            ENABLING_ENABLED
        );
    }

    #[test]
    fn cancel_transition() {
        assert_eq!(DISABLED.cancel_transition(), DISABLED);
        assert_eq!(ENABLED.cancel_transition(), ENABLED);
        assert_eq!(ENABLING_DISABLED.cancel_transition(), DISABLED);
        assert_eq!(ENABLING_ENABLED.cancel_transition(), ENABLED);
        assert_eq!(DISABLING_DISABLED.cancel_transition(), DISABLED);
        assert_eq!(DISABLING_ENABLED.cancel_transition(), ENABLED);
    }
}
