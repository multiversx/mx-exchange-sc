use elrond_wasm::types::{Address, BigInt, ManagedVec, MultiValueEncoded};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_buffer, managed_token_id, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    tx_mock::TxResult,
    DebugApi,
};
use energy_factory_mock::EnergyFactoryMock;
use energy_query::Energy;
use governance_v2::{
    configurable::ConfigurablePropertiesModule, proposal_storage::VoteType, GovernanceV2,
};

pub const MIN_ENERGY_FOR_PROPOSE: u64 = 500;
pub const MIN_FEE_FOR_PROPOSE: u64 = 1_000;
pub const QUORUM: u64 = 1_500;
pub const VOTING_DELAY_BLOCKS: u64 = 10;
pub const VOTING_PERIOD_BLOCKS: u64 = 20;
pub const LOCKING_PERIOD_BLOCKS: u64 = 30;
pub static LKMEX_TOKEN_ID: &[u8] = b"LKMEX-123456";

pub const USER_ENERGY: u64 = 1_000;
pub const GAS_LIMIT: u64 = 1_000_000;

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
    pub gov_wrapper: ContractObjWrapper<governance_v2::ContractObj<DebugApi>, GovBuilder>,
    pub current_block: u64,
}

impl<GovBuilder> GovSetup<GovBuilder>
where
    GovBuilder: 'static + Copy + Fn() -> governance_v2::ContractObj<DebugApi>,
{
    pub fn new(gov_builder: GovBuilder) -> Self {
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let first_user = b_mock.create_user_account(&rust_zero);
        let second_user = b_mock.create_user_account(&rust_zero);
        let third_user = b_mock.create_user_account(&rust_zero);

        // init energy factory
        let energy_factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            energy_factory_mock::contract_obj,
            "energy factory path",
        );
        b_mock
            .execute_tx(&owner, &energy_factory_wrapper, &rust_zero, |sc| {
                sc.init();
                sc.user_energy(&managed_address!(&first_user))
                    .set(&Energy::new(
                        BigInt::from(managed_biguint!(USER_ENERGY)),
                        0,
                        managed_biguint!(0),
                    ));
                sc.user_energy(&managed_address!(&second_user))
                    .set(&Energy::new(
                        BigInt::from(managed_biguint!(USER_ENERGY)),
                        0,
                        managed_biguint!(0),
                    ));
                sc.user_energy(&managed_address!(&third_user))
                    .set(&Energy::new(
                        BigInt::from(managed_biguint!(USER_ENERGY + 1u64)),
                        0,
                        managed_biguint!(0),
                    ));
            })
            .assert_ok();

        // init governance sc
        let gov_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner), gov_builder, "gov path");

        b_mock
            .execute_tx(&owner, &gov_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_biguint!(MIN_ENERGY_FOR_PROPOSE),
                    managed_biguint!(MIN_FEE_FOR_PROPOSE),
                    managed_biguint!(QUORUM),
                    VOTING_DELAY_BLOCKS,
                    VOTING_PERIOD_BLOCKS,
                    LOCKING_PERIOD_BLOCKS,
                    managed_address!(energy_factory_wrapper.address_ref()),
                );
            })
            .assert_ok();

        b_mock
            .execute_tx(&owner, &gov_wrapper, &rust_zero, |sc| {
                sc.fee_token_id().set(managed_token_id!(LKMEX_TOKEN_ID));
            })
            .assert_ok();

        b_mock.set_block_nonce(10);

        Self {
            b_mock,
            owner,
            first_user,
            second_user,
            third_user,
            gov_wrapper,
            current_block: 10,
        }
    }

    pub fn propose(
        &mut self,
        proposer: &Address,
        fee_amount: u64,
        dest_address: &Address,
        endpoint_name: &[u8],
        args: Vec<Vec<u8>>,
    ) -> (TxResult, usize) {
        let mut proposal_id = 0;
        let result = self.b_mock.execute_esdt_transfer(
            proposer,
            &self.gov_wrapper,
            LKMEX_TOKEN_ID,
            1u64,
            &rust_biguint!(fee_amount),
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

                proposal_id = sc.propose(managed_buffer!(b"change quorum"), actions);
            },
        );

        (result, proposal_id)
    }

    pub fn up_vote(&mut self, voter: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(voter, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.vote(proposal_id, VoteType::UpVote);
            })
    }

    pub fn down_vote(&mut self, voter: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(voter, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.vote(proposal_id, VoteType::DownVote);
            })
    }

    pub fn down_veto_vote(&mut self, voter: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(voter, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.vote(proposal_id, VoteType::DownVetoVote);
            })
    }

    pub fn abstain_vote(&mut self, voter: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(voter, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.vote(proposal_id, VoteType::AbstainVote);
            })
    }

    pub fn queue(&mut self, proposal_id: usize) -> TxResult {
        self.b_mock.execute_tx(
            &self.first_user,
            &self.gov_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.queue(proposal_id);
            },
        )
    }

    pub fn execute(&mut self, proposal_id: usize) -> TxResult {
        self.b_mock.execute_tx(
            &self.first_user,
            &self.gov_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.execute(proposal_id);
            },
        )
    }

    pub fn cancel(&mut self, caller: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(caller, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.cancel(proposal_id);
            })
    }

    pub fn deposit_tokens(
        &mut self,
        caller: &Address,
        amount: u64,
        proposal_id: usize,
    ) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            caller,
            &self.gov_wrapper,
            LKMEX_TOKEN_ID,
            1u64,
            &rust_biguint!(amount),
            |sc| {
                sc.deposit_tokens_for_proposal(proposal_id);
            },
        )
    }

    pub fn claim_deposited_tokens(&mut self, caller: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(caller, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.claim_deposited_tokens(proposal_id);
            })
    }

    pub fn increment_block_nonce(&mut self, inc_amount: u64) {
        self.current_block += inc_amount;
        self.b_mock.set_block_nonce(self.current_block);
    }

    pub fn set_block_nonce(&mut self, block_nonce: u64) {
        self.current_block = block_nonce;
        self.b_mock.set_block_nonce(self.current_block);
    }
}
