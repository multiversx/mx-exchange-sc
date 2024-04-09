#![allow(deprecated)]

use governance_v2::{
    proposal_storage::{ProposalStorageModule, VoteType},
    GovernanceV2,
};
use multiversx_sc::{
    arrayvec::ArrayVec,
    hex_literal::hex,
    types::{Address, ManagedByteArray, ManagedVec, MultiValueEncoded},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_buffer, managed_token_id, rust_biguint,
    whitebox_legacy::TxResult,
    whitebox_legacy::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};
use num_bigint::BigUint;

pub const MIN_FEE_FOR_PROPOSE: u64 = 3_000_000;
pub const QUORUM_PERCENTAGE: u64 = 5000;
pub const VOTING_DELAY_BLOCKS: u64 = 10;
pub const VOTING_PERIOD_BLOCKS: u64 = 14_500;
pub const LOCKING_PERIOD_BLOCKS: u64 = 30;
pub const WITHDRAW_PERCENTAGE: u64 = 5_000; // 50%
pub const WXMEX_TOKEN_ID: &[u8] = b"WXMEX-123456";
pub const DECIMALS_CONST: u64 = 1_000_000_000_000_000_000;
pub const GAS_LIMIT: u64 = 1_000_000u64;
pub const FIRST_USER_QUORUM: u64 = 217_433_990_694u64;
pub const SECOND_USER_QUORUM: u64 = 59_024_824_840u64;
pub const THIRD_USER_QUORUM: u64 = 40_000_000_000u64;

#[derive(Clone)]
pub struct Payment {
    pub token: Vec<u8>,
    pub nonce: u64,
    pub amount: u64,
}

pub struct GovSetup<GovBuilder>
where
    GovBuilder: 'static + Copy + Fn() -> governance_v2::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub first_user: Address,
    pub second_user: Address,
    pub third_user: Address,
    pub first_merkle_user: Address,
    pub second_merkle_user: Address,
    pub third_merkle_user: Address,
    pub gov_wrapper: ContractObjWrapper<governance_v2::ContractObj<DebugApi>, GovBuilder>,
    pub current_block: u64,
}

impl<GovBuilder> GovSetup<GovBuilder>
where
    GovBuilder: 'static + Copy + Fn() -> governance_v2::ContractObj<DebugApi>,
{
    pub fn new(gov_builder: GovBuilder) -> Self {
        let _ = DebugApi::dummy();
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let first_user = b_mock.create_user_account(&rust_zero);
        let second_user = b_mock.create_user_account(&rust_zero);
        let third_user = b_mock.create_user_account(&rust_zero);
        let first_merkle_user = Address::from(&hex!(
            "25ee243280fc6e740424a28fa40c795458943b475cd77f3778f9c8e0b4a1e7f8"
        ));
        let second_merkle_user = Address::from(&hex!(
            "0d5acc0ee5a229ae549dad903fb7bcbc1f80b67198949f02fd611d25d41689cb"
        ));
        let third_merkle_user = Address::from(&hex!(
            "190c55b8f27547244c65ad13cbbe7457d5fb90481f34b84160a1cf6e44e0875c"
        ));
        b_mock.create_user_account_fixed_address(&first_merkle_user, &rust_zero);

        // init governance sc
        let gov_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner), gov_builder, "gov path");

        // let min_fee = managed_biguint!(MIN_FEE_FOR_PROPOSE )* managed_biguint!(DECIMALS_CONST);
        b_mock
            .execute_tx(&owner, &gov_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_biguint!(MIN_FEE_FOR_PROPOSE) * DECIMALS_CONST,
                    managed_biguint!(QUORUM_PERCENTAGE),
                    VOTING_DELAY_BLOCKS,
                    VOTING_PERIOD_BLOCKS,
                    WITHDRAW_PERCENTAGE,
                    managed_token_id!(WXMEX_TOKEN_ID),
                );
            })
            .assert_ok();

        Self {
            b_mock,
            owner,
            first_user,
            second_user,
            third_user,
            first_merkle_user,
            second_merkle_user,
            third_merkle_user,
            gov_wrapper,
            current_block: 0,
        }
    }

    pub fn propose(
        &mut self,
        root_hash: ManagedByteArray<DebugApi, 32>,
        proposer: &Address,
        fee_amount: &BigUint,
        dest_address: &Address,
        endpoint_name: &[u8],
        args: Vec<Vec<u8>>,
    ) -> (TxResult, usize) {
        let mut proposal_id = 0;
        let result = self.b_mock.execute_esdt_transfer(
            proposer,
            &self.gov_wrapper,
            WXMEX_TOKEN_ID,
            1u64,
            fee_amount,
            |sc| {
                let mut args_managed = ManagedVec::new();
                for arg in args {
                    args_managed.push(managed_buffer!(&arg));
                }

                let mut actions = MultiValueEncoded::new();
                actions.push(
                    (
                        GAS_LIMIT,
                        managed_address!(dest_address),
                        managed_buffer!(endpoint_name),
                        args_managed,
                    )
                        .into(),
                );
                let total_quorum = managed_biguint!(FIRST_USER_QUORUM)
                    + managed_biguint!(SECOND_USER_QUORUM)
                    + managed_biguint!(THIRD_USER_QUORUM);
                proposal_id = sc.propose(
                    root_hash,
                    total_quorum,
                    managed_buffer!(b"changeTODO"),
                    actions,
                );
            },
        );

        (result, proposal_id)
    }

    pub fn up_vote(
        &mut self,
        voter: &Address,
        power: &BigUint,
        proof: &ArrayVec<ManagedByteArray<DebugApi, 32>, 18>,
        proposal_id: usize,
    ) -> TxResult {
        self.b_mock
            .execute_tx(voter, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.vote(proposal_id, VoteType::UpVote, power.into(), proof.clone());
            })
    }

    pub fn down_vote(
        &mut self,
        voter: &Address,
        power: &BigUint,
        proof: &ArrayVec<ManagedByteArray<DebugApi, 32>, 18>,
        proposal_id: usize,
    ) -> TxResult {
        self.b_mock
            .execute_tx(voter, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.vote(proposal_id, VoteType::DownVote, power.into(), proof.clone());
            })
    }

    pub fn down_veto_vote(
        &mut self,
        voter: &Address,
        power: &BigUint,
        proof: &ArrayVec<ManagedByteArray<DebugApi, 32>, 18>,
        proposal_id: usize,
    ) -> TxResult {
        self.b_mock
            .execute_tx(voter, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.vote(
                    proposal_id,
                    VoteType::DownVetoVote,
                    power.into(),
                    proof.clone(),
                );
            })
    }

    pub fn abstain_vote(
        &mut self,
        voter: &Address,
        power: &BigUint,
        proof: &ArrayVec<ManagedByteArray<DebugApi, 32>, 18>,
        proposal_id: usize,
    ) -> TxResult {
        self.b_mock
            .execute_tx(voter, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.vote(
                    proposal_id,
                    VoteType::AbstainVote,
                    power.into(),
                    proof.clone(),
                );
            })
    }

    pub fn withdraw_after_defeated(&mut self, caller: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(caller, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.withdraw_deposit(proposal_id);
            })
    }

    pub fn cancel_proposal(&mut self, caller: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(caller, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.cancel(proposal_id);
            })
    }

    pub fn check_proposal_id_consistency(
        &mut self,
        caller: &Address,
        proposal_id: usize,
    ) -> TxResult {
        self.b_mock
            .execute_tx(caller, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                let proposal = sc.proposals().get(proposal_id);
                assert!(
                    proposal.proposal_id == proposal_id,
                    "Proposal ID is inconsistent!"
                )
            })
    }
    pub fn increment_block_nonce(&mut self, inc_amount: u64) {
        self.current_block += inc_amount;
        self.b_mock.set_block_nonce(self.current_block);
    }
    pub fn first_merkle_proof(&self) -> ArrayVec<ManagedByteArray<DebugApi, 32>, 18> {
        ArrayVec::from([
            ManagedByteArray::from(&hex!(
                "5e9e904152b2a06dafc26aa02b8c55b2ec3370cdf55b06b15fe8c94bc56e43fc"
            )),
            ManagedByteArray::from(&hex!(
                "47aa547fa524519bc946d0883591e9d273a65b4a8f06a37baf170c707c4fb782"
            )),
            ManagedByteArray::from(&hex!(
                "da05a587448779b9368680f0ee745b3ff2df1132e309a2de518aa9814cc25de8"
            )),
            ManagedByteArray::from(&hex!(
                "14ae69620e727d8e899119ef7a1f04191f15f7c704c2673445510a1bbce7edb2"
            )),
            ManagedByteArray::from(&hex!(
                "449e8beb9a901efc2815253a2ce416e3b98ff771fdfc9e35f4ce31004003e533"
            )),
            ManagedByteArray::from(&hex!(
                "fdf33783f8de396173fd37b558213117bb2be3430d50b18eeb5da1af0b98fd66"
            )),
            ManagedByteArray::from(&hex!(
                "889a7d403faae66ec51db6b0708d889ca277e2fd64f78e47612c07077df6eccc"
            )),
            ManagedByteArray::from(&hex!(
                "5cc55493d05c9bf63431384f184e0cc6ab13c14509c3e9a64dc49b47e3821c6f"
            )),
            ManagedByteArray::from(&hex!(
                "ca559e91dee09fccf8d3dbf9bda81f57d4094faecf2f9ac3a98e84b3f376c81c"
            )),
            ManagedByteArray::from(&hex!(
                "781cf1316f87a336a7219095aa8b523a821d260ca0b7a7f94af5c769e17c8f82"
            )),
            ManagedByteArray::from(&hex!(
                "5e97339ee010d2ae3ac54c38c7158e7a7ef4311f89b28b4634d0d35fd4097dc0"
            )),
            ManagedByteArray::from(&hex!(
                "2c7b2103c3e48386a7eda174cec726c28ec6ce80466deaac49dbaf469f35d059"
            )),
            ManagedByteArray::from(&hex!(
                "9b2ceb7809f078724efb71ea3542885c990985d41746d4cab7948928384c3a4c"
            )),
            ManagedByteArray::from(&hex!(
                "b1f37e46c3f84a3c68804d62b26b7f035c90f344252d77fdeff2393793ae34d4"
            )),
            ManagedByteArray::from(&hex!(
                "95ead464db14a9a65c3b9c5378cf927b76ff530ae0762dbb6456bfeb467f97b2"
            )),
            ManagedByteArray::from(&hex!(
                "39e10ed734ddaa3098edc6300edeed2fce75780fc718140359b13c58045a1838"
            )),
            ManagedByteArray::from(&hex!(
                "b36fbf7a645b24bab3eaca5c351985f6bbb95723b47db78171dd7d7f8883ecad"
            )),
            ManagedByteArray::from(&hex!(
                "bffe20aa722c488465e18d10ac3abe3002603bbd4c535211d9bd9b34ce7259a1"
            )),
        ])
    }

    pub fn second_merkle_proof(&self) -> ArrayVec<ManagedByteArray<DebugApi, 32>, 18> {
        ArrayVec::from([
            ManagedByteArray::from(&hex!(
                "b1d7a256ae6b35cce14497b3735d71dd205f099f1a035a7a5cb96e8bf5c32f31"
            )),
            ManagedByteArray::from(&hex!(
                "81dc00d137001723d5654e3120d63601b08ca2b4b8dc41802476529e6d6ada9e"
            )),
            ManagedByteArray::from(&hex!(
                "985e3554adfefea5d2c3f7c93d404fd547cfead9054ce313845656507037df40"
            )),
            ManagedByteArray::from(&hex!(
                "14ae69620e727d8e899119ef7a1f04191f15f7c704c2673445510a1bbce7edb2"
            )),
            ManagedByteArray::from(&hex!(
                "449e8beb9a901efc2815253a2ce416e3b98ff771fdfc9e35f4ce31004003e533"
            )),
            ManagedByteArray::from(&hex!(
                "fdf33783f8de396173fd37b558213117bb2be3430d50b18eeb5da1af0b98fd66"
            )),
            ManagedByteArray::from(&hex!(
                "889a7d403faae66ec51db6b0708d889ca277e2fd64f78e47612c07077df6eccc"
            )),
            ManagedByteArray::from(&hex!(
                "5cc55493d05c9bf63431384f184e0cc6ab13c14509c3e9a64dc49b47e3821c6f"
            )),
            ManagedByteArray::from(&hex!(
                "ca559e91dee09fccf8d3dbf9bda81f57d4094faecf2f9ac3a98e84b3f376c81c"
            )),
            ManagedByteArray::from(&hex!(
                "781cf1316f87a336a7219095aa8b523a821d260ca0b7a7f94af5c769e17c8f82"
            )),
            ManagedByteArray::from(&hex!(
                "5e97339ee010d2ae3ac54c38c7158e7a7ef4311f89b28b4634d0d35fd4097dc0"
            )),
            ManagedByteArray::from(&hex!(
                "2c7b2103c3e48386a7eda174cec726c28ec6ce80466deaac49dbaf469f35d059"
            )),
            ManagedByteArray::from(&hex!(
                "9b2ceb7809f078724efb71ea3542885c990985d41746d4cab7948928384c3a4c"
            )),
            ManagedByteArray::from(&hex!(
                "b1f37e46c3f84a3c68804d62b26b7f035c90f344252d77fdeff2393793ae34d4"
            )),
            ManagedByteArray::from(&hex!(
                "95ead464db14a9a65c3b9c5378cf927b76ff530ae0762dbb6456bfeb467f97b2"
            )),
            ManagedByteArray::from(&hex!(
                "39e10ed734ddaa3098edc6300edeed2fce75780fc718140359b13c58045a1838"
            )),
            ManagedByteArray::from(&hex!(
                "b36fbf7a645b24bab3eaca5c351985f6bbb95723b47db78171dd7d7f8883ecad"
            )),
            ManagedByteArray::from(&hex!(
                "bffe20aa722c488465e18d10ac3abe3002603bbd4c535211d9bd9b34ce7259a1"
            )),
        ])
    }

    pub fn third_merkle_proof(&self) -> ArrayVec<ManagedByteArray<DebugApi, 32>, 18> {
        ArrayVec::from([
            ManagedByteArray::from(&hex!(
                "aa68ae7eac4de3cc643717d33ab7d2e1b098788b127f298f23a34e8720ec609d"
            )),
            ManagedByteArray::from(&hex!(
                "e3db2cf9f85b49279eba44b0f62ac2c6bcdf870f198b9484344634a58860ba2d"
            )),
            ManagedByteArray::from(&hex!(
                "64468b8481d2c1737977e7382eaaa12c8072ee76fda33c3b6aaa193b38d4f1f2"
            )),
            ManagedByteArray::from(&hex!(
                "6159dc0c80183e89221a36db3fedfabe38933d0db73c1156ad32d8caf0537085"
            )),
            ManagedByteArray::from(&hex!(
                "449e8beb9a901efc2815253a2ce416e3b98ff771fdfc9e35f4ce31004003e533"
            )),
            ManagedByteArray::from(&hex!(
                "fdf33783f8de396173fd37b558213117bb2be3430d50b18eeb5da1af0b98fd66"
            )),
            ManagedByteArray::from(&hex!(
                "889a7d403faae66ec51db6b0708d889ca277e2fd64f78e47612c07077df6eccc"
            )),
            ManagedByteArray::from(&hex!(
                "5cc55493d05c9bf63431384f184e0cc6ab13c14509c3e9a64dc49b47e3821c6f"
            )),
            ManagedByteArray::from(&hex!(
                "ca559e91dee09fccf8d3dbf9bda81f57d4094faecf2f9ac3a98e84b3f376c81c"
            )),
            ManagedByteArray::from(&hex!(
                "781cf1316f87a336a7219095aa8b523a821d260ca0b7a7f94af5c769e17c8f82"
            )),
            ManagedByteArray::from(&hex!(
                "5e97339ee010d2ae3ac54c38c7158e7a7ef4311f89b28b4634d0d35fd4097dc0"
            )),
            ManagedByteArray::from(&hex!(
                "2c7b2103c3e48386a7eda174cec726c28ec6ce80466deaac49dbaf469f35d059"
            )),
            ManagedByteArray::from(&hex!(
                "9b2ceb7809f078724efb71ea3542885c990985d41746d4cab7948928384c3a4c"
            )),
            ManagedByteArray::from(&hex!(
                "b1f37e46c3f84a3c68804d62b26b7f035c90f344252d77fdeff2393793ae34d4"
            )),
            ManagedByteArray::from(&hex!(
                "95ead464db14a9a65c3b9c5378cf927b76ff530ae0762dbb6456bfeb467f97b2"
            )),
            ManagedByteArray::from(&hex!(
                "39e10ed734ddaa3098edc6300edeed2fce75780fc718140359b13c58045a1838"
            )),
            ManagedByteArray::from(&hex!(
                "b36fbf7a645b24bab3eaca5c351985f6bbb95723b47db78171dd7d7f8883ecad"
            )),
            ManagedByteArray::from(&hex!(
                "bffe20aa722c488465e18d10ac3abe3002603bbd4c535211d9bd9b34ce7259a1"
            )),
        ])
    }

    pub fn get_first_user_voting_power(&self) -> BigUint {
        BigUint::from(217_433_990_694u64)
    }

    pub fn get_second_user_voting_power(&self) -> BigUint {
        BigUint::from(59_024_824_840u64)
    }

    pub fn get_third_user_voting_power(&self) -> BigUint {
        BigUint::from(40_000_000_000u64)
    }

    pub fn get_merkle_root_hash(&self) -> ManagedByteArray<DebugApi, 32> {
        ManagedByteArray::from(&hex!(
            "0fdb09afb35351d5becc3a79dd9bf03bae7c2366d186a6c8e8276f545d024ef5"
        ))
    }
}
