use pausable::State;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait CommonMethodsModule {
    #[inline]
    fn is_state_active(&self, state: State) -> bool {
        state == State::Active || state == State::PartialActive
    }
}
