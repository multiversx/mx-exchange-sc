elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::config;

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq)]
pub enum VoteType {
    Upvote = 1,
    DownVote = 2,
}

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq)]
pub struct VoteNFTAttributes<M: ManagedTypeApi> {
    proposal_id: u64,
    vote_type: VoteType,
    vote_weight: BigUint<M>,
    payment: EsdtTokenPayment<M>,
}

#[elrond_wasm::module]
pub trait VoteHelper: config::Config {
    fn create_vote_nft(
        &self,
        _proposal_id: u64,
        _vote_type: VoteType,
        _vote_weight: BigUint,
        _payment: EsdtTokenPayment<Self::Api>,
    ) -> EsdtTokenPayment<Self::Api> {
        todo!()
    }
}
