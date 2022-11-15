use elrond_wasm::io::load_endpoint_args;
use elrond_wasm::{
    contract_base::{CallableContract, ContractBase},
    types::{ContractCall, ManagedAddress, ManagedBuffer},
};
use elrond_wasm_debug::DebugApi;

static DEPOSIT_FN_NAME: &[u8] = b"depositUserTokens";
static FINISH_UNSTAKE_FN_NAME: &[u8] = b"finalizeUnstake";

#[derive(Clone)]
pub struct UnbondScMock {}

impl ContractBase for UnbondScMock {
    type Api = DebugApi;
}

impl CallableContract for UnbondScMock {
    fn call(&self, fn_name: &[u8]) -> bool {
        if fn_name == DEPOSIT_FN_NAME {
            self.send_to_user();
            true
        } else {
            false
        }
    }

    fn clone_obj(&self) -> Box<dyn CallableContract> {
        Box::new(self.clone())
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

        // let dest_user_addr_raw = <Self as ContractBase>::Api::argument_api_impl()
        //     .tx_input_box
        //     .args[0]
        //     .clone();
        // let dest_user =
        //     ManagedAddress::try_from(ManagedBuffer::new_from_bytes(&dest_user_addr_raw)).unwrap();

        self.send().direct_esdt(
            &dest_user,
            &unlocked_tokens.token_identifier,
            unlocked_tokens.token_nonce,
            &unlocked_tokens.amount,
        );

        let penalty_amount = &locked_tokens.amount - &unlocked_tokens.amount;
        if penalty_amount > 0 {
            let energy_sc = self.blockchain().get_caller();
            let contract_call = ContractCall::<DebugApi, ()>::new(
                energy_sc,
                ManagedBuffer::new_from_bytes(FINISH_UNSTAKE_FN_NAME),
            )
            .add_esdt_token_transfer(
                locked_tokens.token_identifier,
                locked_tokens.token_nonce,
                penalty_amount,
            );
            let _: () = contract_call.execute_on_dest_context();
        }
    }
}
