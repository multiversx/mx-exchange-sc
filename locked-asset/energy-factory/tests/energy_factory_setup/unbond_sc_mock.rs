use multiversx_sc::io::load_endpoint_args;
use multiversx_sc::{
    contract_base::{CallableContract, ContractBase},
    types::ManagedAddress,
};
use multiversx_sc_scenario::DebugApi;

static DEPOSIT_USER_TOKENS_FN_NAME: &str = "depositUserTokens";
static DEPOSIT_FEES_FN_NAME: &str = "depositFees";

#[derive(Clone)]
pub struct UnbondScMock {}

impl ContractBase for UnbondScMock {
    type Api = DebugApi;
}

impl CallableContract for UnbondScMock {
    fn call(&self, fn_name: &str) -> bool {
        if fn_name == DEPOSIT_USER_TOKENS_FN_NAME {
            self.send_to_user();
            true
        } else {
            fn_name == DEPOSIT_FEES_FN_NAME
        }
    }
}

impl UnbondScMock {
    pub fn new() -> Self {
        UnbondScMock {}
    }

    // We don't test cancel unbond here, so we simply send to the user
    pub fn send_to_user(&self) {
        let [locked_tokens, unlocked_tokens] = self.call_value().multi_esdt();

        let locked_tokens_burn_amount = unlocked_tokens.amount.clone();
        self.send().esdt_local_burn(
            &locked_tokens.token_identifier,
            locked_tokens.token_nonce,
            &locked_tokens_burn_amount,
        );

        let (dest_user, ()) =
            load_endpoint_args::<DebugApi, (ManagedAddress<DebugApi>, ())>(("dest_user", ()));

        self.send().direct_esdt(
            &dest_user,
            &unlocked_tokens.token_identifier,
            unlocked_tokens.token_nonce,
            &unlocked_tokens.amount,
        );
    }
}
