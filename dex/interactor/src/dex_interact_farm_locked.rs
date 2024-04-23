use multiversx_sc_scenario::imports::{BigUint, ManagedAddress, ReturnsResult, TokenIdentifier};
use multiversx_sc_snippets::InteractorPrepareAsync;

use crate::{dex_interact_pair::InteractorToken, farm_with_locked_rewards_proxy, DexInteract};

impl DexInteract {
    pub async fn enter_farm(
        &mut self,
        lp_token: InteractorToken,
    ) -> (InteractorToken, InteractorToken) {
        println!("Attempting to enter farm with locked rewards...");

        let result_token = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_farm_with_locked_rewards_address())
            .gas(100_000_000u64)
            .typed(farm_with_locked_rewards_proxy::FarmProxy)
            .enter_farm_endpoint(ManagedAddress::from(self.wallet_address.as_address()))
            .payment((
                TokenIdentifier::from(lp_token.token_id.as_bytes()),
                lp_token.nonce,
                BigUint::from(lp_token.amount),
            ))
            .returns(ReturnsResult)
            .prepare_async()
            .run()
            .await;
        (
            InteractorToken::from(result_token.0 .0),
            InteractorToken::from(result_token.0 .1),
        )
    }
}
