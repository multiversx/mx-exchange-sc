#![feature(prelude_import)]
#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]
#[prelude_import]
use core::prelude::rust_2018::*;
#[macro_use]
extern crate core;
#[macro_use]
extern crate compiler_builtins;
pub mod contexts {
    pub mod base {
        use crate::State;
        use common_structs::FarmTokenAttributes;
        use core::ops::{
            Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div,
            DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub,
            SubAssign,
        };
        use elrond_wasm::{
            api::{
                BigIntApi, BlockchainApi, CallValueApi, CryptoApi, EllipticCurveApi, ErrorApi,
                LogApi, ManagedTypeApi, PrintApi, SendApi,
            },
            arrayvec::ArrayVec,
            contract_base::{ContractBase, ProxyObjBase},
            elrond_codec::{DecodeError, NestedDecode, NestedEncode, TopDecode},
            err_msg,
            esdt::*,
            io::*,
            non_zero_usize,
            non_zero_util::*,
            only_owner, require, sc_error,
            storage::mappers::*,
            types::{
                SCResult::{Err, Ok},
                *,
            },
            Box, Vec,
        };
        use elrond_wasm::{
            derive::{ManagedVecItem, TypeAbi},
            elrond_codec,
            elrond_codec::elrond_codec_derive::{
                NestedDecode, NestedEncode, TopDecode, TopDecodeOrDefault, TopEncode,
                TopEncodeOrDefault,
            },
        };
        use farm_token::FarmToken;
        pub trait Context<M: ManagedTypeApi> {
            fn set_contract_state(&mut self, contract_state: State);
            fn get_contract_state(&self) -> &State;
            fn set_farm_token_id(&mut self, farm_token_id: TokenIdentifier<M>);
            fn get_farm_token_id(&self) -> &TokenIdentifier<M>;
            fn set_farming_token_id(&mut self, farming_token_id: TokenIdentifier<M>);
            fn get_farming_token_id(&self) -> &TokenIdentifier<M>;
            fn set_reward_token_id(&mut self, reward_token_id: TokenIdentifier<M>);
            fn get_reward_token_id(&self) -> &TokenIdentifier<M>;
            fn set_block_nonce(&mut self, nonce: u64);
            fn get_block_nonce(&self) -> u64;
            fn set_block_epoch(&mut self, nonce: u64);
            fn get_block_epoch(&self) -> u64;
            fn set_reward_per_share(&mut self, rps: BigUint<M>);
            fn get_reward_per_share(&self) -> &BigUint<M>;
            fn set_farm_token_supply(&mut self, supply: BigUint<M>);
            fn get_farm_token_supply(&self) -> &BigUint<M>;
            fn set_division_safety_constant(&mut self, dsc: BigUint<M>);
            fn get_division_safety_constant(&self) -> &BigUint<M>;
            fn set_reward_reserve(&mut self, reward_reserve: BigUint<M>);
            fn get_reward_reserve(&self) -> &BigUint<M>;
            fn increase_reward_reserve(&mut self, amount: &BigUint<M>);
            fn decrease_reward_reserve(&mut self, amount: &BigUint<M>);
            fn update_reward_per_share(&mut self, reward_added: &BigUint<M>);
            fn set_input_attributes(&mut self, attrs: FarmTokenAttributes<M>);
            fn get_input_attributes(&self) -> Option<&FarmTokenAttributes<M>>;
            fn set_initial_farming_amount(&mut self, amount: BigUint<M>);
            fn get_initial_farming_amount(&self) -> Option<&BigUint<M>>;
            fn set_position_reward(&mut self, amount: BigUint<M>);
            fn get_position_reward(&self) -> Option<&BigUint<M>>;
            fn get_storage_cache(&self) -> &StorageCache<M>;
            fn set_final_reward(&mut self, payment: EsdtTokenPayment<M>);
            fn get_final_reward(&self) -> Option<&EsdtTokenPayment<M>>;
            fn was_output_created_with_merge(&self) -> bool;
            fn get_output_attributes(&self) -> Option<&FarmTokenAttributes<M>>;
            fn set_output_position(&mut self, position: FarmToken<M>, created_with_merge: bool);
            fn get_caller(&self) -> &ManagedAddress<M>;
            fn set_output_payments(&mut self, payments: ManagedVec<M, EsdtTokenPayment<M>>);
            fn get_output_payments(&self) -> &ManagedVec<M, EsdtTokenPayment<M>>;
            fn get_opt_accept_funds_func(&self) -> &OptionalArg<ManagedBuffer<M>>;
            fn get_tx_input(&self) -> &dyn TxInput<M>;
        }
        pub trait TxInput<M: ManagedTypeApi> {
            fn get_args(&self) -> &dyn TxInputArgs<M>;
            fn get_payments(&self) -> &dyn TxInputPayments<M>;
            fn is_valid(&self) -> bool;
        }
        pub trait TxInputArgs<M: ManagedTypeApi> {
            fn are_valid(&self) -> bool;
        }
        pub trait TxInputPayments<M: ManagedTypeApi> {
            fn are_valid(&self) -> bool;
            fn get_first(&self) -> &EsdtTokenPayment<M>;
            fn get_additional(&self) -> Option<&ManagedVec<M, EsdtTokenPayment<M>>>;
        }
        pub struct StorageCache<M: ManagedTypeApi> {
            pub contract_state: State,
            pub farm_token_id: TokenIdentifier<M>,
            pub farming_token_id: TokenIdentifier<M>,
            pub reward_token_id: TokenIdentifier<M>,
            pub reward_reserve: BigUint<M>,
            pub reward_per_share: BigUint<M>,
            pub farm_token_supply: BigUint<M>,
            pub division_safety_constant: BigUint<M>,
        }
        impl<M: ManagedTypeApi> Default for StorageCache<M> {
            fn default() -> Self {
                StorageCache {
                    contract_state: State::Inactive,
                    farm_token_id: TokenIdentifier::egld(),
                    farming_token_id: TokenIdentifier::egld(),
                    reward_token_id: TokenIdentifier::egld(),
                    reward_reserve: BigUint::zero(),
                    reward_per_share: BigUint::zero(),
                    farm_token_supply: BigUint::zero(),
                    division_safety_constant: BigUint::zero(),
                }
            }
        }
    }
    pub mod claim_rewards {
        use super::base::*;
        use crate::State;
        use common_structs::FarmTokenAttributes;
        use core::ops::{
            Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div,
            DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub,
            SubAssign,
        };
        use elrond_wasm::{
            api::{
                BigIntApi, BlockchainApi, CallValueApi, CryptoApi, EllipticCurveApi, ErrorApi,
                LogApi, ManagedTypeApi, PrintApi, SendApi,
            },
            arrayvec::ArrayVec,
            contract_base::{ContractBase, ProxyObjBase},
            elrond_codec::{DecodeError, NestedDecode, NestedEncode, TopDecode},
            err_msg,
            esdt::*,
            io::*,
            non_zero_usize,
            non_zero_util::*,
            only_owner, require, sc_error,
            storage::mappers::*,
            types::{
                SCResult::{Err, Ok},
                *,
            },
            Box, Vec,
        };
        use elrond_wasm::{
            derive::{ManagedVecItem, TypeAbi},
            elrond_codec,
            elrond_codec::elrond_codec_derive::{
                NestedDecode, NestedEncode, TopDecode, TopDecodeOrDefault, TopEncode,
                TopEncodeOrDefault,
            },
        };
        use farm_token::FarmToken;
        pub struct ClaimRewardsContext<M: ManagedTypeApi> {
            caller: ManagedAddress<M>,
            tx_input: ClaimRewardsTxInput<M>,
            block_nonce: u64,
            block_epoch: u64,
            position_reward: BigUint<M>,
            storage_cache: StorageCache<M>,
            initial_farming_amount: BigUint<M>,
            final_reward: Option<EsdtTokenPayment<M>>,
            output_attributes: Option<FarmTokenAttributes<M>>,
            output_created_with_merge: bool,
            output_payments: ManagedVec<M, EsdtTokenPayment<M>>,
        }
        pub struct ClaimRewardsTxInput<M: ManagedTypeApi> {
            args: ClaimRewardsArgs<M>,
            payments: ClaimRewardsPayments<M>,
            attributes: Option<FarmTokenAttributes<M>>,
        }
        pub struct ClaimRewardsArgs<M: ManagedTypeApi> {
            opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>,
        }
        pub struct ClaimRewardsPayments<M: ManagedTypeApi> {
            first_payment: EsdtTokenPayment<M>,
            additional_payments: ManagedVec<M, EsdtTokenPayment<M>>,
        }
        impl<M: ManagedTypeApi> ClaimRewardsTxInput<M> {
            pub fn new(args: ClaimRewardsArgs<M>, payments: ClaimRewardsPayments<M>) -> Self {
                ClaimRewardsTxInput {
                    args,
                    payments,
                    attributes: None,
                }
            }
        }
        impl<M: ManagedTypeApi> ClaimRewardsArgs<M> {
            pub fn new(opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>) -> Self {
                ClaimRewardsArgs {
                    opt_accept_funds_func,
                }
            }
        }
        impl<M: ManagedTypeApi> ClaimRewardsPayments<M> {
            pub fn new(
                first_payment: EsdtTokenPayment<M>,
                additional_payments: ManagedVec<M, EsdtTokenPayment<M>>,
            ) -> Self {
                ClaimRewardsPayments {
                    first_payment,
                    additional_payments,
                }
            }
        }
        impl<M: ManagedTypeApi> ClaimRewardsContext<M> {
            pub fn new(tx_input: ClaimRewardsTxInput<M>, caller: ManagedAddress<M>) -> Self {
                ClaimRewardsContext {
                    caller,
                    tx_input,
                    block_nonce: 0,
                    block_epoch: 0,
                    position_reward: BigUint::zero(),
                    storage_cache: StorageCache::default(),
                    initial_farming_amount: BigUint::zero(),
                    final_reward: None,
                    output_attributes: None,
                    output_created_with_merge: true,
                    output_payments: ManagedVec::new(),
                }
            }
        }
        impl<M: ManagedTypeApi> Context<M> for ClaimRewardsContext<M> {
            #[inline]
            fn set_contract_state(&mut self, contract_state: State) {
                self.storage_cache.contract_state = contract_state;
            }
            #[inline]
            fn get_contract_state(&self) -> &State {
                &self.storage_cache.contract_state
            }
            #[inline]
            fn get_caller(&self) -> &ManagedAddress<M> {
                &self.caller
            }
            #[inline]
            fn set_output_payments(&mut self, payments: ManagedVec<M, EsdtTokenPayment<M>>) {
                self.output_payments = payments
            }
            #[inline]
            fn get_output_payments(&self) -> &ManagedVec<M, EsdtTokenPayment<M>> {
                &self.output_payments
            }
            #[inline]
            fn get_opt_accept_funds_func(&self) -> &OptionalArg<ManagedBuffer<M>> {
                &self.tx_input.args.opt_accept_funds_func
            }
            #[inline]
            fn get_tx_input(&self) -> &dyn TxInput<M> {
                &self.tx_input
            }
            #[inline]
            fn set_farm_token_id(&mut self, farm_token_id: TokenIdentifier<M>) {
                self.storage_cache.farm_token_id = farm_token_id
            }
            #[inline]
            fn get_farm_token_id(&self) -> &TokenIdentifier<M> {
                &self.storage_cache.farm_token_id
            }
            #[inline]
            fn set_farming_token_id(&mut self, farming_token_id: TokenIdentifier<M>) {
                self.storage_cache.farming_token_id = farming_token_id
            }
            #[inline]
            fn get_farming_token_id(&self) -> &TokenIdentifier<M> {
                &self.storage_cache.farming_token_id
            }
            #[inline]
            fn set_reward_token_id(&mut self, reward_token_id: TokenIdentifier<M>) {
                self.storage_cache.reward_token_id = reward_token_id;
            }
            #[inline]
            fn get_reward_token_id(&self) -> &TokenIdentifier<M> {
                &self.storage_cache.reward_token_id
            }
            #[inline]
            fn set_block_nonce(&mut self, nonce: u64) {
                self.block_nonce = nonce;
            }
            #[inline]
            fn get_block_nonce(&self) -> u64 {
                self.block_nonce
            }
            #[inline]
            fn set_block_epoch(&mut self, epoch: u64) {
                self.block_epoch = epoch;
            }
            #[inline]
            fn get_block_epoch(&self) -> u64 {
                self.block_epoch
            }
            #[inline]
            fn set_reward_per_share(&mut self, rps: BigUint<M>) {
                self.storage_cache.reward_per_share = rps;
            }
            #[inline]
            fn get_reward_per_share(&self) -> &BigUint<M> {
                &self.storage_cache.reward_per_share
            }
            #[inline]
            fn set_farm_token_supply(&mut self, supply: BigUint<M>) {
                self.storage_cache.farm_token_supply = supply;
            }
            #[inline]
            fn get_farm_token_supply(&self) -> &BigUint<M> {
                &self.storage_cache.farm_token_supply
            }
            #[inline]
            fn set_division_safety_constant(&mut self, dsc: BigUint<M>) {
                self.storage_cache.division_safety_constant = dsc;
            }
            #[inline]
            fn get_division_safety_constant(&self) -> &BigUint<M> {
                &self.storage_cache.division_safety_constant
            }
            #[inline]
            fn set_reward_reserve(&mut self, rr: BigUint<M>) {
                self.storage_cache.reward_reserve = rr;
            }
            #[inline]
            fn get_reward_reserve(&self) -> &BigUint<M> {
                &self.storage_cache.reward_reserve
            }
            #[inline]
            fn increase_reward_reserve(&mut self, amount: &BigUint<M>) {
                self.storage_cache.reward_reserve += amount;
            }
            #[inline]
            fn decrease_reward_reserve(&mut self, amount: &BigUint<M>) {
                self.storage_cache.reward_reserve -= amount;
            }
            #[inline]
            fn update_reward_per_share(&mut self, reward_added: &BigUint<M>) {
                if self.storage_cache.farm_token_supply != 0u64 {
                    self.storage_cache.reward_per_share += reward_added
                        * &self.storage_cache.division_safety_constant
                        / &self.storage_cache.farm_token_supply;
                }
            }
            #[inline]
            fn get_storage_cache(&self) -> &StorageCache<M> {
                &self.storage_cache
            }
            #[inline]
            fn set_input_attributes(&mut self, _attr: FarmTokenAttributes<M>) {}
            #[inline]
            fn get_input_attributes(&self) -> Option<&FarmTokenAttributes<M>> {
                self.tx_input.attributes.as_ref()
            }
            #[inline]
            fn set_position_reward(&mut self, amount: BigUint<M>) {
                self.position_reward = amount;
            }
            #[inline]
            fn get_position_reward(&self) -> Option<&BigUint<M>> {
                Some(&self.position_reward)
            }
            #[inline]
            fn set_initial_farming_amount(&mut self, amount: BigUint<M>) {
                self.initial_farming_amount = amount;
            }
            #[inline]
            fn get_initial_farming_amount(&self) -> Option<&BigUint<M>> {
                Some(&self.initial_farming_amount)
            }
            #[inline]
            fn set_final_reward(&mut self, payment: EsdtTokenPayment<M>) {
                self.final_reward = Some(payment);
            }
            #[inline]
            fn get_final_reward(&self) -> Option<&EsdtTokenPayment<M>> {
                self.final_reward.as_ref()
            }
            #[inline]
            fn was_output_created_with_merge(&self) -> bool {
                self.output_created_with_merge
            }
            #[inline]
            fn get_output_attributes(&self) -> Option<&FarmTokenAttributes<M>> {
                self.output_attributes.as_ref()
            }
            #[inline]
            fn set_output_position(&mut self, position: FarmToken<M>, created_with_merge: bool) {
                self.output_payments.push(position.token_amount);
                self.output_created_with_merge = true;
                self.output_attributes = Some(position.attributes);
            }
        }
        impl<M: ManagedTypeApi> TxInputArgs<M> for ClaimRewardsArgs<M> {
            #[inline]
            fn are_valid(&self) -> bool {
                true
            }
        }
        impl<M: ManagedTypeApi> TxInputPayments<M> for ClaimRewardsPayments<M> {
            #[inline]
            fn are_valid(&self) -> bool {
                true
            }
            #[inline]
            fn get_first(&self) -> &EsdtTokenPayment<M> {
                &self.first_payment
            }
            #[inline]
            fn get_additional(&self) -> Option<&ManagedVec<M, EsdtTokenPayment<M>>> {
                Some(&self.additional_payments)
            }
        }
        impl<M: ManagedTypeApi> ClaimRewardsPayments<M> {}
        impl<M: ManagedTypeApi> TxInput<M> for ClaimRewardsTxInput<M> {
            #[inline]
            fn get_args(&self) -> &dyn TxInputArgs<M> {
                &self.args
            }
            #[inline]
            fn get_payments(&self) -> &dyn TxInputPayments<M> {
                &self.payments
            }
            #[inline]
            fn is_valid(&self) -> bool {
                true
            }
        }
        impl<M: ManagedTypeApi> ClaimRewardsTxInput<M> {}
        impl<M: ManagedTypeApi> ClaimRewardsContext<M> {
            pub fn is_accepted_payment(&self) -> bool {
                let first_payment_pass = self.tx_input.payments.first_payment.token_identifier
                    == self.storage_cache.farm_token_id
                    && self.tx_input.payments.first_payment.token_nonce != 0
                    && self.tx_input.payments.first_payment.amount != 0u64;
                if !first_payment_pass {
                    return false;
                }
                for payment in self.tx_input.payments.additional_payments.iter() {
                    let payment_pass = payment.token_identifier == self.storage_cache.farm_token_id
                        && payment.token_nonce != 0
                        && payment.amount != 0;
                    if !payment_pass {
                        return false;
                    }
                }
                true
            }
            #[inline]
            pub fn was_output_created_with_merge(&self) -> bool {
                self.output_created_with_merge
            }
            #[inline]
            pub fn get_output_attributes(&self) -> &FarmTokenAttributes<M> {
                self.output_attributes.as_ref().unwrap()
            }
            #[inline]
            pub fn set_output_position(
                &mut self,
                position: FarmToken<M>,
                created_with_merge: bool,
            ) {
                self.output_payments.push(position.token_amount);
                self.output_created_with_merge = true;
                self.output_attributes = Some(position.attributes);
            }
            #[inline]
            pub fn decrease_reward_reserve(&self) {
                self.storage_cache.reward_reserve -= &self.position_reward
            }
        }
    }
    pub mod compound_rewards {
        use super::base::*;
        use crate::State;
        use common_structs::FarmTokenAttributes;
        use core::ops::{
            Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div,
            DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub,
            SubAssign,
        };
        use elrond_wasm::{
            api::{
                BigIntApi, BlockchainApi, CallValueApi, CryptoApi, EllipticCurveApi, ErrorApi,
                LogApi, ManagedTypeApi, PrintApi, SendApi,
            },
            arrayvec::ArrayVec,
            contract_base::{ContractBase, ProxyObjBase},
            elrond_codec::{DecodeError, NestedDecode, NestedEncode, TopDecode},
            err_msg,
            esdt::*,
            io::*,
            non_zero_usize,
            non_zero_util::*,
            only_owner, require, sc_error,
            storage::mappers::*,
            types::{
                SCResult::{Err, Ok},
                *,
            },
            Box, Vec,
        };
        use elrond_wasm::{
            derive::{ManagedVecItem, TypeAbi},
            elrond_codec,
            elrond_codec::elrond_codec_derive::{
                NestedDecode, NestedEncode, TopDecode, TopDecodeOrDefault, TopEncode,
                TopEncodeOrDefault,
            },
        };
        use farm_token::FarmToken;
        pub struct CompoundRewardsContext<M: ManagedTypeApi> {
            caller: ManagedAddress<M>,
            tx_input: CompoundRewardsTxInput<M>,
            block_nonce: u64,
            block_epoch: u64,
            position_reward: BigUint<M>,
            storage_cache: StorageCache<M>,
            initial_farming_amount: BigUint<M>,
            final_reward: Option<EsdtTokenPayment<M>>,
            output_attributes: Option<FarmTokenAttributes<M>>,
            output_created_with_merge: bool,
            output_payments: ManagedVec<M, EsdtTokenPayment<M>>,
        }
        pub struct CompoundRewardsTxInput<M: ManagedTypeApi> {
            args: CompoundRewardsArgs<M>,
            payments: CompoundRewardsPayments<M>,
            attributes: Option<FarmTokenAttributes<M>>,
        }
        pub struct CompoundRewardsArgs<M: ManagedTypeApi> {
            opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>,
        }
        pub struct CompoundRewardsPayments<M: ManagedTypeApi> {
            first_payment: EsdtTokenPayment<M>,
            additional_payments: ManagedVec<M, EsdtTokenPayment<M>>,
        }
        impl<M: ManagedTypeApi> CompoundRewardsTxInput<M> {
            pub fn new(args: CompoundRewardsArgs<M>, payments: CompoundRewardsPayments<M>) -> Self {
                CompoundRewardsTxInput {
                    args,
                    payments,
                    attributes: None,
                }
            }
        }
        impl<M: ManagedTypeApi> CompoundRewardsArgs<M> {
            pub fn new(opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>) -> Self {
                CompoundRewardsArgs {
                    opt_accept_funds_func,
                }
            }
        }
        impl<M: ManagedTypeApi> CompoundRewardsPayments<M> {
            pub fn new(
                first_payment: EsdtTokenPayment<M>,
                additional_payments: ManagedVec<M, EsdtTokenPayment<M>>,
            ) -> Self {
                CompoundRewardsPayments {
                    first_payment,
                    additional_payments,
                }
            }
        }
        impl<M: ManagedTypeApi> CompoundRewardsContext<M> {
            pub fn new(tx_input: CompoundRewardsTxInput<M>, caller: ManagedAddress<M>) -> Self {
                CompoundRewardsContext {
                    caller,
                    tx_input,
                    block_nonce: 0,
                    block_epoch: 0,
                    position_reward: BigUint::zero(),
                    storage_cache: StorageCache::default(),
                    initial_farming_amount: BigUint::zero(),
                    final_reward: None,
                    output_attributes: None,
                    output_created_with_merge: true,
                    output_payments: ManagedVec::new(),
                }
            }
        }
        impl<M: ManagedTypeApi> Context<M> for CompoundRewardsContext<M> {
            #[inline]
            fn set_contract_state(&mut self, contract_state: State) {
                self.storage_cache.contract_state = contract_state;
            }
            #[inline]
            fn get_contract_state(&self) -> &State {
                &self.storage_cache.contract_state
            }
            #[inline]
            fn get_caller(&self) -> &ManagedAddress<M> {
                &self.caller
            }
            #[inline]
            fn set_output_payments(&mut self, payments: ManagedVec<M, EsdtTokenPayment<M>>) {
                self.output_payments = payments
            }
            #[inline]
            fn get_output_payments(&self) -> &ManagedVec<M, EsdtTokenPayment<M>> {
                &self.output_payments
            }
            #[inline]
            fn get_opt_accept_funds_func(&self) -> &OptionalArg<ManagedBuffer<M>> {
                &self.tx_input.args.opt_accept_funds_func
            }
            #[inline]
            fn get_tx_input(&self) -> &dyn TxInput<M> {
                &self.tx_input
            }
            #[inline]
            fn set_farm_token_id(&mut self, farm_token_id: TokenIdentifier<M>) {
                self.storage_cache.farm_token_id = farm_token_id
            }
            #[inline]
            fn get_farm_token_id(&self) -> &TokenIdentifier<M> {
                &self.storage_cache.farm_token_id
            }
            #[inline]
            fn set_farming_token_id(&mut self, farming_token_id: TokenIdentifier<M>) {
                self.storage_cache.farming_token_id = farming_token_id
            }
            #[inline]
            fn get_farming_token_id(&self) -> &TokenIdentifier<M> {
                &self.storage_cache.farming_token_id
            }
            #[inline]
            fn set_reward_token_id(&mut self, reward_token_id: TokenIdentifier<M>) {
                self.storage_cache.reward_token_id = reward_token_id;
            }
            #[inline]
            fn get_reward_token_id(&self) -> &TokenIdentifier<M> {
                &self.storage_cache.reward_token_id
            }
            #[inline]
            fn set_block_nonce(&mut self, nonce: u64) {
                self.block_nonce = nonce;
            }
            #[inline]
            fn get_block_nonce(&self) -> u64 {
                self.block_nonce
            }
            #[inline]
            fn set_block_epoch(&mut self, epoch: u64) {
                self.block_epoch = epoch;
            }
            #[inline]
            fn get_block_epoch(&self) -> u64 {
                self.block_epoch
            }
            #[inline]
            fn set_reward_per_share(&mut self, rps: BigUint<M>) {
                self.storage_cache.reward_per_share = rps;
            }
            #[inline]
            fn get_reward_per_share(&self) -> &BigUint<M> {
                &self.storage_cache.reward_per_share
            }
            #[inline]
            fn set_farm_token_supply(&mut self, supply: BigUint<M>) {
                self.storage_cache.farm_token_supply = supply;
            }
            #[inline]
            fn get_farm_token_supply(&self) -> &BigUint<M> {
                &self.storage_cache.farm_token_supply
            }
            #[inline]
            fn set_division_safety_constant(&mut self, dsc: BigUint<M>) {
                self.storage_cache.division_safety_constant = dsc;
            }
            #[inline]
            fn get_division_safety_constant(&self) -> &BigUint<M> {
                &self.storage_cache.division_safety_constant
            }
            #[inline]
            fn set_reward_reserve(&mut self, rr: BigUint<M>) {
                self.storage_cache.reward_reserve = rr;
            }
            #[inline]
            fn get_reward_reserve(&self) -> &BigUint<M> {
                &self.storage_cache.reward_reserve
            }
            #[inline]
            fn increase_reward_reserve(&mut self, amount: &BigUint<M>) {
                self.storage_cache.reward_reserve += amount;
            }
            #[inline]
            fn decrease_reward_reserve(&mut self, amount: &BigUint<M>) {
                self.storage_cache.reward_reserve -= amount;
            }
            #[inline]
            fn update_reward_per_share(&mut self, reward_added: &BigUint<M>) {
                if self.storage_cache.farm_token_supply != 0u64 {
                    self.storage_cache.reward_per_share += reward_added
                        * &self.storage_cache.division_safety_constant
                        / &self.storage_cache.farm_token_supply;
                }
            }
            #[inline]
            fn get_storage_cache(&self) -> &StorageCache<M> {
                &self.storage_cache
            }
            #[inline]
            fn set_input_attributes(&mut self, _attr: FarmTokenAttributes<M>) {}
            #[inline]
            fn get_input_attributes(&self) -> Option<&FarmTokenAttributes<M>> {
                self.tx_input.attributes.as_ref()
            }
            #[inline]
            fn set_position_reward(&mut self, amount: BigUint<M>) {
                self.position_reward = amount;
            }
            #[inline]
            fn get_position_reward(&self) -> Option<&BigUint<M>> {
                Some(&self.position_reward)
            }
            #[inline]
            fn set_initial_farming_amount(&mut self, amount: BigUint<M>) {
                self.initial_farming_amount = amount;
            }
            #[inline]
            fn get_initial_farming_amount(&self) -> Option<&BigUint<M>> {
                Some(&self.initial_farming_amount)
            }
            #[inline]
            fn set_final_reward(&mut self, payment: EsdtTokenPayment<M>) {
                self.final_reward = Some(payment);
            }
            #[inline]
            fn get_final_reward(&self) -> Option<&EsdtTokenPayment<M>> {
                self.final_reward.as_ref()
            }
            #[inline]
            fn was_output_created_with_merge(&self) -> bool {
                self.output_created_with_merge
            }
            #[inline]
            fn get_output_attributes(&self) -> Option<&FarmTokenAttributes<M>> {
                self.output_attributes.as_ref()
            }
            #[inline]
            fn set_output_position(&mut self, position: FarmToken<M>, created_with_merge: bool) {
                self.output_payments.push(position.token_amount);
                self.output_created_with_merge = true;
                self.output_attributes = Some(position.attributes);
            }
        }
        impl<M: ManagedTypeApi> TxInputArgs<M> for CompoundRewardsArgs<M> {
            #[inline]
            fn are_valid(&self) -> bool {
                true
            }
        }
        impl<M: ManagedTypeApi> TxInputPayments<M> for CompoundRewardsPayments<M> {
            #[inline]
            fn are_valid(&self) -> bool {
                true
            }
            #[inline]
            fn get_first(&self) -> &EsdtTokenPayment<M> {
                &self.first_payment
            }
            #[inline]
            fn get_additional(&self) -> Option<&ManagedVec<M, EsdtTokenPayment<M>>> {
                Some(&self.additional_payments)
            }
        }
        impl<M: ManagedTypeApi> CompoundRewardsPayments<M> {}
        impl<M: ManagedTypeApi> TxInput<M> for CompoundRewardsTxInput<M> {
            #[inline]
            fn get_args(&self) -> &dyn TxInputArgs<M> {
                &self.args
            }
            #[inline]
            fn get_payments(&self) -> &dyn TxInputPayments<M> {
                &self.payments
            }
            #[inline]
            fn is_valid(&self) -> bool {
                true
            }
        }
        impl<M: ManagedTypeApi> CompoundRewardsTxInput<M> {}
        impl<M: ManagedTypeApi> CompoundRewardsContext<M> {
            pub fn is_accepted_payment(&self) -> bool {
                let first_payment_pass = self.tx_input.payments.first_payment.token_identifier
                    == self.storage_cache.farm_token_id
                    && self.tx_input.payments.first_payment.token_nonce != 0
                    && self.tx_input.payments.first_payment.amount != 0u64;
                if !first_payment_pass {
                    return false;
                }
                for payment in self.tx_input.payments.additional_payments.iter() {
                    let payment_pass = payment.token_identifier == self.storage_cache.farm_token_id
                        && payment.token_nonce != 0
                        && payment.amount != 0;
                    if !payment_pass {
                        return false;
                    }
                }
                true
            }
            #[inline]
            pub fn was_output_created_with_merge(&self) -> bool {
                self.output_created_with_merge
            }
            #[inline]
            pub fn get_output_attributes(&self) -> &FarmTokenAttributes<M> {
                self.output_attributes.as_ref().unwrap()
            }
            #[inline]
            pub fn set_output_position(
                &mut self,
                position: FarmToken<M>,
                created_with_merge: bool,
            ) {
                self.output_payments.push(position.token_amount);
                self.output_created_with_merge = true;
                self.output_attributes = Some(position.attributes);
            }
            #[inline]
            pub fn decrease_reward_reserve(&self) {
                self.storage_cache.reward_reserve -= &self.position_reward
            }
        }
    }
    pub mod ctx_helper {
        use super::base::*;
        use super::claim_rewards::*;
        use super::compound_rewards::*;
        use super::enter_farm::*;
        use super::exit_farm::*;
        use crate::assert;
        use crate::errors::*;
        use core::ops::{
            Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div,
            DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub,
            SubAssign,
        };
        use elrond_wasm::{
            api::{
                BigIntApi, BlockchainApi, CallValueApi, CryptoApi, EllipticCurveApi, ErrorApi,
                LogApi, ManagedTypeApi, PrintApi, SendApi,
            },
            arrayvec::ArrayVec,
            contract_base::{ContractBase, ProxyObjBase},
            elrond_codec::{DecodeError, NestedDecode, NestedEncode, TopDecode},
            err_msg,
            esdt::*,
            io::*,
            non_zero_usize,
            non_zero_util::*,
            only_owner, require, sc_error,
            storage::mappers::*,
            types::{
                SCResult::{Err, Ok},
                *,
            },
            Box, Vec,
        };
        use elrond_wasm::{
            derive::{ManagedVecItem, TypeAbi},
            elrond_codec,
            elrond_codec::elrond_codec_derive::{
                NestedDecode, NestedEncode, TopDecode, TopDecodeOrDefault, TopEncode,
                TopEncodeOrDefault,
            },
        };
        pub trait CtxHelper:
            elrond_wasm::contract_base::ContractBase
            + Sized
            + config::ConfigModule
            + token_send::TokenSendModule
            + rewards::RewardsModule
            + farm_token::FarmTokenModule
            + token_merge::TokenMergeModule
        {
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn new_enter_farm_context(
                &self,
                opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
            ) -> EnterFarmContext<Self::Api> {
                let caller = self.blockchain().get_caller();
                let payments = self.call_value().all_esdt_transfers();
                let mut payments_iter = payments.iter();
                let first_payment = payments_iter.next().unwrap();
                let mut additional_payments = ManagedVec::new();
                while let Some(payment) = payments_iter.next() {
                    additional_payments.push(payment);
                }
                let args = EnterFarmArgs::new(opt_accept_funds_func);
                let payments = EnterFarmPayments::new(first_payment, additional_payments);
                let tx = EnterFarmTxInput::new(args, payments);
                EnterFarmContext::new(tx, caller)
            }
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn new_claim_rewards_context(
                &self,
                opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
            ) -> ClaimRewardsContext<Self::Api> {
                let caller = self.blockchain().get_caller();
                let payments = self.call_value().all_esdt_transfers();
                let mut payments_iter = payments.iter();
                let first_payment = payments_iter.next().unwrap();
                let mut additional_payments = ManagedVec::new();
                while let Some(payment) = payments_iter.next() {
                    additional_payments.push(payment);
                }
                let args = ClaimRewardsArgs::new(opt_accept_funds_func);
                let payments = ClaimRewardsPayments::new(first_payment, additional_payments);
                let tx = ClaimRewardsTxInput::new(args, payments);
                ClaimRewardsContext::new(tx, caller)
            }
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn new_compound_rewards_context(
                &self,
                opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
            ) -> CompoundRewardsContext<Self::Api> {
                let caller = self.blockchain().get_caller();
                let payments = self.call_value().all_esdt_transfers();
                let mut payments_iter = payments.iter();
                let first_payment = payments_iter.next().unwrap();
                let mut additional_payments = ManagedVec::new();
                while let Some(payment) = payments_iter.next() {
                    additional_payments.push(payment);
                }
                let args = CompoundRewardsArgs::new(opt_accept_funds_func);
                let payments = CompoundRewardsPayments::new(first_payment, additional_payments);
                let tx = CompoundRewardsTxInput::new(args, payments);
                CompoundRewardsContext::new(tx, caller)
            }
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn new_exit_farm_context(
                &self,
                opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
            ) -> ExitFarmContext<Self::Api> {
                let caller = self.blockchain().get_caller();
                let payments = self.call_value().all_esdt_transfers();
                let mut payments_iter = payments.iter();
                let first_payment = payments_iter.next().unwrap();
                if !payments_iter.next().is_none() {
                    self.raw_vm_api().signal_error(ERROR_BAD_PAYMENTS_LEN)
                };
                let args = ExitFarmArgs::new(opt_accept_funds_func);
                let payments = ExitFarmPayments::new(first_payment);
                let tx = ExitFarmTxInput::new(args, payments);
                ExitFarmContext::new(tx, caller)
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn load_state(&self, context: &mut dyn Context<Self::Api>) {
                context.set_contract_state(self.state().get());
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn load_farm_token_id(&self, context: &mut dyn Context<Self::Api>) {
                context.set_farm_token_id(self.farm_token_id().get());
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn load_farming_token_id(&self, context: &mut dyn Context<Self::Api>) {
                context.set_farming_token_id(self.farming_token_id().get());
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn load_reward_token_id(&self, context: &mut dyn Context<Self::Api>) {
                context.set_reward_token_id(self.reward_token_id().get());
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn load_block_nonce(&self, context: &mut dyn Context<Self::Api>) {
                context.set_reward_token_id(self.reward_token_id().get());
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn load_block_epoch(&self, context: &mut dyn Context<Self::Api>) {
                context.set_reward_token_id(self.reward_token_id().get());
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn load_reward_reserve(&self, context: &mut dyn Context<Self::Api>) {
                context.set_reward_reserve(self.reward_reserve().get());
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn load_reward_per_share(&self, context: &mut dyn Context<Self::Api>) {
                context.set_reward_per_share(self.reward_per_share().get());
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn load_farm_token_supply(&self, context: &mut dyn Context<Self::Api>) {
                context.set_farm_token_supply(self.farm_token_supply().get());
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn load_division_safety_constant(&self, context: &mut dyn Context<Self::Api>) {
                context.set_division_safety_constant(self.division_safety_constant().get());
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn commit_changes(&self, context: &dyn Context<Self::Api>) {
                self.reward_reserve().set(context.get_reward_per_share());
                self.reward_per_share().set(context.get_reward_per_share());
                self.farm_token_supply()
                    .set(context.get_farm_token_supply());
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn execute_output_payments(&self, context: &dyn Context<Self::Api>) {
                let result = self.send_multiple_tokens_if_not_zero(
                    context.get_caller(),
                    context.get_output_payments(),
                    context.get_opt_accept_funds_func(),
                );
                if !result.is_ok() {
                    self.raw_vm_api().signal_error(ERROR_PAYMENT_FAILED)
                };
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn load_farm_attributes(&self, context: &mut dyn Context<Self::Api>) {
                let farm_token_id = context.get_farm_token_id().clone();
                let nonce = context
                    .get_tx_input()
                    .get_payments()
                    .get_first()
                    .token_nonce;
                context.set_input_attributes(
                    self.blockchain()
                        .get_esdt_token_data(
                            &self.blockchain().get_sc_address(),
                            &farm_token_id,
                            nonce,
                        )
                        .decode_attributes()
                        .unwrap(),
                )
            }
            #[inline]
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn calculate_reward(&self, context: &mut dyn Context<Self::Api>) {
                let reward = if context.get_reward_per_share()
                    > &context
                        .get_input_attributes()
                        .unwrap()
                        .initial_farming_amount
                {
                    context.get_tx_input().get_payments().get_first().amount
                        * &(context.get_reward_per_share()
                            - &context
                                .get_input_attributes()
                                .unwrap()
                                .initial_farming_amount)
                        / context.get_division_safety_constant()
                } else {
                    elrond_wasm::types::BigUint::<Self::Api>::zero()
                };
                context.set_position_reward(reward);
            }
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn calculate_initial_farming_amount(&self, context: &mut dyn Context<Self::Api>) {
                let mut initial_farming_token_amount = self
                    .rule_of_three_non_zero_result(
                        &context.get_tx_input().get_payments().get_first().amount,
                        &context.get_input_attributes().unwrap().current_farm_amount,
                        &context
                            .get_input_attributes()
                            .unwrap()
                            .initial_farming_amount,
                    )
                    .unwrap_or_signal_error(self.type_manager());
                context.set_initial_farming_amount(initial_farming_token_amount);
            }
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn increase_reward_with_compounded_rewards(
                &self,
                context: &mut ExitFarmContext<Self::Api>,
            ) {
                let mut amount = self.rule_of_three(
                    &context.get_tx_input().get_payments().get_first().amount,
                    &context.get_input_attributes().unwrap().current_farm_amount,
                    &context.get_input_attributes().unwrap().compounded_reward,
                );
                context.increase_position_reward(&amount);
            }
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn construct_output_payments_exit(&self, context: &mut ExitFarmContext<Self::Api>) {
                let mut result = ManagedVec::new();
                result.push(self.create_payment(
                    context.get_farming_token_id(),
                    0,
                    context.get_initial_farming_amount().unwrap(),
                ));
                context.set_output_payments(result);
            }
            #[allow(clippy::too_many_arguments)]
            #[allow(clippy::type_complexity)]
            fn construct_and_get_result(
                &self,
                context: &dyn Context<Self::Api>,
            ) -> MultiResult2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>>
            {
                MultiResult2::from((
                    context.get_output_payments().get(0).unwrap(),
                    context.get_final_reward().unwrap().clone(),
                ))
            }
        }
        pub trait AutoImpl: elrond_wasm::contract_base::ContractBase {}
        impl<C> CtxHelper for C where
            C: AutoImpl
                + config::ConfigModule
                + token_send::TokenSendModule
                + rewards::RewardsModule
                + farm_token::FarmTokenModule
                + token_merge::TokenMergeModule
        {
        }
        pub trait EndpointWrappers:
            elrond_wasm::contract_base::ContractBase
            + CtxHelper
            + config::EndpointWrappers
            + token_send::EndpointWrappers
            + rewards::EndpointWrappers
            + farm_token::EndpointWrappers
            + token_merge::EndpointWrappers
        {
            fn call(&self, fn_name: &[u8]) -> bool {
                if match fn_name {
                    b"callBack" => {
                        self::EndpointWrappers::callback(self);
                        return true;
                    }
                    other => false,
                } {
                    return true;
                }
                if config::EndpointWrappers::call(self, fn_name) {
                    return true;
                }
                if token_send::EndpointWrappers::call(self, fn_name) {
                    return true;
                }
                if rewards::EndpointWrappers::call(self, fn_name) {
                    return true;
                }
                if farm_token::EndpointWrappers::call(self, fn_name) {
                    return true;
                }
                if token_merge::EndpointWrappers::call(self, fn_name) {
                    return true;
                }
                false
            }
            fn callback_selector(
                &self,
                mut ___cb_closure___: elrond_wasm::types::CallbackClosureForDeser<Self::Api>,
            ) -> elrond_wasm::types::CallbackSelectorResult<Self::Api> {
                let mut ___call_result_loader___ = EndpointDynArgLoader::new(self.raw_vm_api());
                let ___cb_closure_matcher___ = ___cb_closure___.matcher::<32usize>();
                if ___cb_closure_matcher___.matches_empty() {
                    return elrond_wasm::types::CallbackSelectorResult::Processed;
                }
                match config::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                    elrond_wasm::types::CallbackSelectorResult::Processed => {
                        return elrond_wasm::types::CallbackSelectorResult::Processed;
                    }
                    elrond_wasm::types::CallbackSelectorResult::NotProcessed(
                        recovered_cb_closure,
                    ) => {
                        ___cb_closure___ = recovered_cb_closure;
                    }
                }
                match token_send::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                    elrond_wasm::types::CallbackSelectorResult::Processed => {
                        return elrond_wasm::types::CallbackSelectorResult::Processed;
                    }
                    elrond_wasm::types::CallbackSelectorResult::NotProcessed(
                        recovered_cb_closure,
                    ) => {
                        ___cb_closure___ = recovered_cb_closure;
                    }
                }
                match rewards::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                    elrond_wasm::types::CallbackSelectorResult::Processed => {
                        return elrond_wasm::types::CallbackSelectorResult::Processed;
                    }
                    elrond_wasm::types::CallbackSelectorResult::NotProcessed(
                        recovered_cb_closure,
                    ) => {
                        ___cb_closure___ = recovered_cb_closure;
                    }
                }
                match farm_token::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                    elrond_wasm::types::CallbackSelectorResult::Processed => {
                        return elrond_wasm::types::CallbackSelectorResult::Processed;
                    }
                    elrond_wasm::types::CallbackSelectorResult::NotProcessed(
                        recovered_cb_closure,
                    ) => {
                        ___cb_closure___ = recovered_cb_closure;
                    }
                }
                match token_merge::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                    elrond_wasm::types::CallbackSelectorResult::Processed => {
                        return elrond_wasm::types::CallbackSelectorResult::Processed;
                    }
                    elrond_wasm::types::CallbackSelectorResult::NotProcessed(
                        recovered_cb_closure,
                    ) => {
                        ___cb_closure___ = recovered_cb_closure;
                    }
                }
                elrond_wasm::types::CallbackSelectorResult::NotProcessed(___cb_closure___)
            }
            fn callback(&self) {
                if let Some(___cb_closure___) =
                    elrond_wasm::types::CallbackClosureForDeser::storage_load_and_clear(
                        self.raw_vm_api(),
                    )
                {
                    if let elrond_wasm::types::CallbackSelectorResult::NotProcessed(_) =
                        self::EndpointWrappers::callback_selector(self, ___cb_closure___)
                    {
                        elrond_wasm::api::ErrorApi::signal_error(
                            &self.raw_vm_api(),
                            err_msg::CALLBACK_BAD_FUNC,
                        );
                    }
                }
            }
        }
        pub struct AbiProvider {}
        impl elrond_wasm::contract_base::ContractAbiProvider for AbiProvider {
            type Api = elrond_wasm::api::uncallable::UncallableApi;
            fn abi() -> elrond_wasm::abi::ContractAbi {
                let mut contract_abi = elrond_wasm :: abi :: ContractAbi { build_info : elrond_wasm :: abi :: BuildInfoAbi { contract_crate : elrond_wasm :: abi :: ContractCrateBuildAbi { name : "farm_with_lock" , version : "0.0.0" , } , framework : elrond_wasm :: abi :: FrameworkBuildAbi :: create () , } , docs : & [] , name : "CtxHelper" , constructors : Vec :: new () , endpoints : Vec :: new () , has_callback : false , type_descriptions : < elrond_wasm :: abi :: TypeDescriptionContainerImpl as elrond_wasm :: abi :: TypeDescriptionContainer > :: new () , } ;
                contract_abi
            }
        }
        pub struct ContractObj<A>
        where
            A: elrond_wasm::api::VMApi + Clone + 'static,
        {
            api: A,
        }
        impl<A> elrond_wasm::contract_base::ContractBase for ContractObj<A>
        where
            A: elrond_wasm::api::VMApi + Clone + 'static,
        {
            type Api = A;
            fn raw_vm_api(&self) -> Self::Api {
                self.api.clone()
            }
        }
        impl<A> config::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
        impl<A> token_send::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
        impl<A> rewards::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
        impl<A> farm_token::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
        impl<A> token_merge::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
        impl<A> AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
        impl<A> config::EndpointWrappers for ContractObj<A> where
            A: elrond_wasm::api::VMApi + Clone + 'static
        {
        }
        impl<A> token_send::EndpointWrappers for ContractObj<A> where
            A: elrond_wasm::api::VMApi + Clone + 'static
        {
        }
        impl<A> rewards::EndpointWrappers for ContractObj<A> where
            A: elrond_wasm::api::VMApi + Clone + 'static
        {
        }
        impl<A> farm_token::EndpointWrappers for ContractObj<A> where
            A: elrond_wasm::api::VMApi + Clone + 'static
        {
        }
        impl<A> token_merge::EndpointWrappers for ContractObj<A> where
            A: elrond_wasm::api::VMApi + Clone + 'static
        {
        }
        impl<A> EndpointWrappers for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
        impl<A> elrond_wasm::contract_base::CallableContract<A> for ContractObj<A>
        where
            A: elrond_wasm::api::VMApi + Clone + 'static,
        {
            fn call(&self, fn_name: &[u8]) -> bool {
                EndpointWrappers::call(self, fn_name)
            }
            fn into_api(self: Box<Self>) -> A {
                self.api
            }
        }
        pub fn contract_obj<A>(api: A) -> ContractObj<A>
        where
            A: elrond_wasm::api::VMApi + Clone + 'static,
        {
            ContractObj { api }
        }
        pub use config::endpoints as __endpoints_0__;
        pub use farm_token::endpoints as __endpoints_3__;
        pub use rewards::endpoints as __endpoints_2__;
        pub use token_merge::endpoints as __endpoints_4__;
        pub use token_send::endpoints as __endpoints_1__;
        #[allow(non_snake_case)]
        pub mod endpoints {
            use super::EndpointWrappers;
            pub use super::__endpoints_0__::*;
            pub use super::__endpoints_1__::*;
            pub use super::__endpoints_2__::*;
            pub use super::__endpoints_3__::*;
            pub use super::__endpoints_4__::*;
        }
        pub trait ProxyTrait:
            elrond_wasm::contract_base::ProxyObjBase
            + Sized
            + config::ProxyTrait
            + token_send::ProxyTrait
            + rewards::ProxyTrait
            + farm_token::ProxyTrait
            + token_merge::ProxyTrait
        {
        }
    }
    pub mod enter_farm {
        use super::base::*;
        use crate::State;
        use common_structs::FarmTokenAttributes;
        use core::ops::{
            Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div,
            DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub,
            SubAssign,
        };
        use elrond_wasm::{
            api::{
                BigIntApi, BlockchainApi, CallValueApi, CryptoApi, EllipticCurveApi, ErrorApi,
                LogApi, ManagedTypeApi, PrintApi, SendApi,
            },
            arrayvec::ArrayVec,
            contract_base::{ContractBase, ProxyObjBase},
            elrond_codec::{DecodeError, NestedDecode, NestedEncode, TopDecode},
            err_msg,
            esdt::*,
            io::*,
            non_zero_usize,
            non_zero_util::*,
            only_owner, require, sc_error,
            storage::mappers::*,
            types::{
                SCResult::{Err, Ok},
                *,
            },
            Box, Vec,
        };
        use elrond_wasm::{
            derive::{ManagedVecItem, TypeAbi},
            elrond_codec,
            elrond_codec::elrond_codec_derive::{
                NestedDecode, NestedEncode, TopDecode, TopDecodeOrDefault, TopEncode,
                TopEncodeOrDefault,
            },
        };
        use farm_token::FarmToken;
        pub struct EnterFarmContext<M: ManagedTypeApi> {
            caller: ManagedAddress<M>,
            tx_input: EnterFarmTxInput<M>,
            block_nonce: u64,
            block_epoch: u64,
            storage_cache: StorageCache<M>,
            output_attributes: Option<FarmTokenAttributes<M>>,
            output_created_with_merge: bool,
            output_payments: ManagedVec<M, EsdtTokenPayment<M>>,
        }
        pub struct EnterFarmTxInput<M: ManagedTypeApi> {
            args: EnterFarmArgs<M>,
            payments: EnterFarmPayments<M>,
        }
        pub struct EnterFarmArgs<M: ManagedTypeApi> {
            opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>,
        }
        pub struct EnterFarmPayments<M: ManagedTypeApi> {
            first_payment: EsdtTokenPayment<M>,
            additional_payments: ManagedVec<M, EsdtTokenPayment<M>>,
        }
        impl<M: ManagedTypeApi> EnterFarmTxInput<M> {
            pub fn new(args: EnterFarmArgs<M>, payments: EnterFarmPayments<M>) -> Self {
                EnterFarmTxInput { args, payments }
            }
        }
        impl<M: ManagedTypeApi> EnterFarmArgs<M> {
            pub fn new(opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>) -> Self {
                EnterFarmArgs {
                    opt_accept_funds_func,
                }
            }
        }
        impl<M: ManagedTypeApi> EnterFarmPayments<M> {
            pub fn new(
                first_payment: EsdtTokenPayment<M>,
                additional_payments: ManagedVec<M, EsdtTokenPayment<M>>,
            ) -> Self {
                EnterFarmPayments {
                    first_payment,
                    additional_payments,
                }
            }
        }
        impl<M: ManagedTypeApi> EnterFarmContext<M> {
            pub fn new(tx_input: EnterFarmTxInput<M>, caller: ManagedAddress<M>) -> Self {
                EnterFarmContext {
                    caller,
                    tx_input,
                    block_nonce: 0,
                    block_epoch: 0,
                    storage_cache: StorageCache::default(),
                    output_attributes: None,
                    output_created_with_merge: true,
                    output_payments: ManagedVec::new(),
                }
            }
        }
        impl<M: ManagedTypeApi> Context<M> for EnterFarmContext<M> {
            #[inline]
            fn set_contract_state(&mut self, contract_state: State) {
                self.storage_cache.contract_state = contract_state;
            }
            #[inline]
            fn get_contract_state(&self) -> &State {
                &self.storage_cache.contract_state
            }
            #[inline]
            fn get_caller(&self) -> &ManagedAddress<M> {
                &self.caller
            }
            #[inline]
            fn set_output_payments(&mut self, payments: ManagedVec<M, EsdtTokenPayment<M>>) {
                self.output_payments = payments
            }
            #[inline]
            fn get_output_payments(&self) -> &ManagedVec<M, EsdtTokenPayment<M>> {
                &self.output_payments
            }
            #[inline]
            fn get_opt_accept_funds_func(&self) -> &OptionalArg<ManagedBuffer<M>> {
                &self.tx_input.args.opt_accept_funds_func
            }
            #[inline]
            fn get_tx_input(&self) -> &dyn TxInput<M> {
                &self.tx_input
            }
            #[inline]
            fn set_farm_token_id(&mut self, farm_token_id: TokenIdentifier<M>) {
                self.storage_cache.farm_token_id = farm_token_id
            }
            #[inline]
            fn get_farm_token_id(&self) -> &TokenIdentifier<M> {
                &self.storage_cache.farm_token_id
            }
            #[inline]
            fn set_farming_token_id(&mut self, farming_token_id: TokenIdentifier<M>) {
                self.storage_cache.farming_token_id = farming_token_id
            }
            #[inline]
            fn get_farming_token_id(&self) -> &TokenIdentifier<M> {
                &self.storage_cache.farming_token_id
            }
            #[inline]
            fn set_reward_token_id(&mut self, reward_token_id: TokenIdentifier<M>) {
                self.storage_cache.reward_token_id = reward_token_id;
            }
            #[inline]
            fn get_reward_token_id(&self) -> &TokenIdentifier<M> {
                &self.storage_cache.reward_token_id
            }
            #[inline]
            fn set_block_nonce(&mut self, nonce: u64) {
                self.block_nonce = nonce;
            }
            #[inline]
            fn get_block_nonce(&self) -> u64 {
                self.block_nonce
            }
            #[inline]
            fn set_block_epoch(&mut self, epoch: u64) {
                self.block_epoch = epoch;
            }
            #[inline]
            fn get_block_epoch(&self) -> u64 {
                self.block_epoch
            }
            #[inline]
            fn set_reward_per_share(&mut self, rps: BigUint<M>) {
                self.storage_cache.reward_per_share = rps;
            }
            #[inline]
            fn get_reward_per_share(&self) -> &BigUint<M> {
                &self.storage_cache.reward_per_share
            }
            #[inline]
            fn set_farm_token_supply(&mut self, supply: BigUint<M>) {
                self.storage_cache.farm_token_supply = supply;
            }
            #[inline]
            fn get_farm_token_supply(&self) -> &BigUint<M> {
                &self.storage_cache.farm_token_supply
            }
            #[inline]
            fn set_division_safety_constant(&mut self, dsc: BigUint<M>) {
                self.storage_cache.division_safety_constant = dsc;
            }
            #[inline]
            fn get_division_safety_constant(&self) -> &BigUint<M> {
                &self.storage_cache.division_safety_constant
            }
            #[inline]
            fn set_reward_reserve(&mut self, rr: BigUint<M>) {
                self.storage_cache.reward_reserve = rr;
            }
            #[inline]
            fn get_reward_reserve(&self) -> &BigUint<M> {
                &self.storage_cache.reward_reserve
            }
            #[inline]
            fn increase_reward_reserve(&mut self, amount: &BigUint<M>) {
                self.storage_cache.reward_reserve += amount;
            }
            #[inline]
            fn decrease_reward_reserve(&mut self, amount: &BigUint<M>) {
                self.storage_cache.reward_reserve -= amount;
            }
            #[inline]
            fn update_reward_per_share(&mut self, reward_added: &BigUint<M>) {
                if self.storage_cache.farm_token_supply != 0u64 {
                    self.storage_cache.reward_per_share += reward_added
                        * &self.storage_cache.division_safety_constant
                        / &self.storage_cache.farm_token_supply;
                }
            }
            #[inline]
            fn get_storage_cache(&self) -> &StorageCache<M> {
                &self.storage_cache
            }
            #[inline]
            fn set_input_attributes(&mut self, _attr: FarmTokenAttributes<M>) {}
            #[inline]
            fn get_input_attributes(&self) -> Option<&FarmTokenAttributes<M>> {
                None
            }
            #[inline]
            fn set_initial_farming_amount(&mut self, amount: BigUint<M>) {}
            #[inline]
            fn get_initial_farming_amount(&self) -> Option<&BigUint<M>> {
                None
            }
            #[inline]
            fn set_position_reward(&mut self, _amount: BigUint<M>) {}
            #[inline]
            fn get_position_reward(&self) -> Option<&BigUint<M>> {
                None
            }
            #[inline]
            fn set_final_reward(&mut self, _payment: EsdtTokenPayment<M>) {}
            #[inline]
            fn get_final_reward(&self) -> Option<&EsdtTokenPayment<M>> {
                None
            }
            #[inline]
            fn was_output_created_with_merge(&self) -> bool {
                self.output_created_with_merge
            }
            #[inline]
            fn get_output_attributes(&self) -> Option<&FarmTokenAttributes<M>> {
                self.output_attributes.as_ref()
            }
            #[inline]
            fn set_output_position(&mut self, position: FarmToken<M>, created_with_merge: bool) {
                self.output_payments.push(position.token_amount);
                self.output_created_with_merge = true;
                self.output_attributes = Some(position.attributes);
            }
        }
        impl<M: ManagedTypeApi> TxInputArgs<M> for EnterFarmArgs<M> {
            #[inline]
            fn are_valid(&self) -> bool {
                true
            }
        }
        impl<M: ManagedTypeApi> TxInputPayments<M> for EnterFarmPayments<M> {
            #[inline]
            fn are_valid(&self) -> bool {
                true
            }
            #[inline]
            fn get_first(&self) -> &EsdtTokenPayment<M> {
                &self.first_payment
            }
            #[inline]
            fn get_additional(&self) -> Option<&ManagedVec<M, EsdtTokenPayment<M>>> {
                Some(&self.additional_payments)
            }
        }
        impl<M: ManagedTypeApi> EnterFarmPayments<M> {}
        impl<M: ManagedTypeApi> TxInput<M> for EnterFarmTxInput<M> {
            #[inline]
            fn get_args(&self) -> &dyn TxInputArgs<M> {
                &self.args
            }
            #[inline]
            fn get_payments(&self) -> &dyn TxInputPayments<M> {
                &self.payments
            }
            #[inline]
            fn is_valid(&self) -> bool {
                true
            }
        }
        impl<M: ManagedTypeApi> EnterFarmTxInput<M> {}
        impl<M: ManagedTypeApi> EnterFarmContext<M> {
            pub fn is_accepted_payment(&self) -> bool {
                let first_payment_pass = self.tx_input.payments.first_payment.token_identifier
                    == self.storage_cache.farming_token_id
                    && self.tx_input.payments.first_payment.token_nonce == 0
                    && self.tx_input.payments.first_payment.amount != 0u64;
                if !first_payment_pass {
                    return false;
                }
                for payment in self.tx_input.payments.additional_payments.iter() {
                    let payment_pass = payment.token_identifier == self.storage_cache.farm_token_id
                        && payment.token_nonce != 0
                        && payment.amount != 0;
                    if !payment_pass {
                        return false;
                    }
                }
                true
            }
        }
    }
    pub mod exit_farm {
        use super::base::*;
        use crate::State;
        use common_structs::FarmTokenAttributes;
        use core::ops::{
            Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div,
            DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub,
            SubAssign,
        };
        use elrond_wasm::{
            api::{
                BigIntApi, BlockchainApi, CallValueApi, CryptoApi, EllipticCurveApi, ErrorApi,
                LogApi, ManagedTypeApi, PrintApi, SendApi,
            },
            arrayvec::ArrayVec,
            contract_base::{ContractBase, ProxyObjBase},
            elrond_codec::{DecodeError, NestedDecode, NestedEncode, TopDecode},
            err_msg,
            esdt::*,
            io::*,
            non_zero_usize,
            non_zero_util::*,
            only_owner, require, sc_error,
            storage::mappers::*,
            types::{
                SCResult::{Err, Ok},
                *,
            },
            Box, Vec,
        };
        use elrond_wasm::{
            derive::{ManagedVecItem, TypeAbi},
            elrond_codec,
            elrond_codec::elrond_codec_derive::{
                NestedDecode, NestedEncode, TopDecode, TopDecodeOrDefault, TopEncode,
                TopEncodeOrDefault,
            },
        };
        use farm_token::FarmToken;
        pub struct ExitFarmContext<M: ManagedTypeApi> {
            caller: ManagedAddress<M>,
            tx_input: ExitFarmTxInput<M>,
            block_nonce: u64,
            block_epoch: u64,
            position_reward: BigUint<M>,
            initial_farming_amount: BigUint<M>,
            final_reward: Option<EsdtTokenPayment<M>>,
            storage_cache: StorageCache<M>,
            output_payments: ManagedVec<M, EsdtTokenPayment<M>>,
        }
        pub struct ExitFarmTxInput<M: ManagedTypeApi> {
            args: ExitFarmArgs<M>,
            payments: ExitFarmPayments<M>,
            attributes: Option<FarmTokenAttributes<M>>,
        }
        pub struct ExitFarmArgs<M: ManagedTypeApi> {
            opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>,
        }
        pub struct ExitFarmPayments<M: ManagedTypeApi> {
            first_payment: EsdtTokenPayment<M>,
        }
        impl<M: ManagedTypeApi> ExitFarmTxInput<M> {
            pub fn new(args: ExitFarmArgs<M>, payments: ExitFarmPayments<M>) -> Self {
                ExitFarmTxInput {
                    args,
                    payments,
                    attributes: None,
                }
            }
        }
        impl<M: ManagedTypeApi> ExitFarmArgs<M> {
            pub fn new(opt_accept_funds_func: OptionalArg<ManagedBuffer<M>>) -> Self {
                ExitFarmArgs {
                    opt_accept_funds_func,
                }
            }
        }
        impl<M: ManagedTypeApi> ExitFarmPayments<M> {
            pub fn new(first_payment: EsdtTokenPayment<M>) -> Self {
                ExitFarmPayments { first_payment }
            }
        }
        impl<M: ManagedTypeApi> ExitFarmContext<M> {
            pub fn new(tx_input: ExitFarmTxInput<M>, caller: ManagedAddress<M>) -> Self {
                ExitFarmContext {
                    caller,
                    tx_input,
                    block_nonce: 0,
                    block_epoch: 0,
                    position_reward: BigUint::zero(),
                    initial_farming_amount: BigUint::zero(),
                    final_reward: None,
                    storage_cache: StorageCache::default(),
                    output_payments: ManagedVec::new(),
                }
            }
        }
        impl<M: ManagedTypeApi> Context<M> for ExitFarmContext<M> {
            #[inline]
            fn set_contract_state(&mut self, contract_state: State) {
                self.storage_cache.contract_state = contract_state;
            }
            #[inline]
            fn get_contract_state(&self) -> &State {
                &self.storage_cache.contract_state
            }
            #[inline]
            fn get_caller(&self) -> &ManagedAddress<M> {
                &self.caller
            }
            #[inline]
            fn set_output_payments(&mut self, payments: ManagedVec<M, EsdtTokenPayment<M>>) {
                self.output_payments = payments
            }
            #[inline]
            fn get_output_payments(&self) -> &ManagedVec<M, EsdtTokenPayment<M>> {
                &self.output_payments
            }
            #[inline]
            fn get_opt_accept_funds_func(&self) -> &OptionalArg<ManagedBuffer<M>> {
                &self.tx_input.args.opt_accept_funds_func
            }
            #[inline]
            fn get_tx_input(&self) -> &dyn TxInput<M> {
                &self.tx_input
            }
            #[inline]
            fn set_farm_token_id(&mut self, farm_token_id: TokenIdentifier<M>) {
                self.storage_cache.farm_token_id = farm_token_id
            }
            #[inline]
            fn get_farm_token_id(&self) -> &TokenIdentifier<M> {
                &self.storage_cache.farm_token_id
            }
            #[inline]
            fn set_farming_token_id(&mut self, farming_token_id: TokenIdentifier<M>) {
                self.storage_cache.farming_token_id = farming_token_id
            }
            #[inline]
            fn get_farming_token_id(&self) -> &TokenIdentifier<M> {
                &self.storage_cache.farming_token_id
            }
            #[inline]
            fn set_reward_token_id(&mut self, reward_token_id: TokenIdentifier<M>) {
                self.storage_cache.reward_token_id = reward_token_id;
            }
            #[inline]
            fn get_reward_token_id(&self) -> &TokenIdentifier<M> {
                &self.storage_cache.reward_token_id
            }
            #[inline]
            fn set_block_nonce(&mut self, nonce: u64) {
                self.block_nonce = nonce;
            }
            #[inline]
            fn get_block_nonce(&self) -> u64 {
                self.block_nonce
            }
            #[inline]
            fn set_block_epoch(&mut self, epoch: u64) {
                self.block_epoch = epoch;
            }
            #[inline]
            fn get_block_epoch(&self) -> u64 {
                self.block_epoch
            }
            #[inline]
            fn set_reward_per_share(&mut self, rps: BigUint<M>) {
                self.storage_cache.reward_per_share = rps;
            }
            #[inline]
            fn get_reward_per_share(&self) -> &BigUint<M> {
                &self.storage_cache.reward_per_share
            }
            #[inline]
            fn set_farm_token_supply(&mut self, supply: BigUint<M>) {
                self.storage_cache.farm_token_supply = supply;
            }
            #[inline]
            fn get_farm_token_supply(&self) -> &BigUint<M> {
                &self.storage_cache.farm_token_supply
            }
            #[inline]
            fn set_division_safety_constant(&mut self, dsc: BigUint<M>) {
                self.storage_cache.division_safety_constant = dsc;
            }
            #[inline]
            fn get_division_safety_constant(&self) -> &BigUint<M> {
                &self.storage_cache.division_safety_constant
            }
            #[inline]
            fn set_reward_reserve(&mut self, rr: BigUint<M>) {
                self.storage_cache.reward_reserve = rr;
            }
            #[inline]
            fn get_reward_reserve(&self) -> &BigUint<M> {
                &self.storage_cache.reward_reserve
            }
            #[inline]
            fn increase_reward_reserve(&mut self, amount: &BigUint<M>) {
                self.storage_cache.reward_reserve += amount;
            }
            #[inline]
            fn decrease_reward_reserve(&mut self, amount: &BigUint<M>) {
                self.storage_cache.reward_reserve -= amount;
            }
            #[inline]
            fn update_reward_per_share(&mut self, reward_added: &BigUint<M>) {
                if self.storage_cache.farm_token_supply != 0u64 {
                    self.storage_cache.reward_per_share += reward_added
                        * &self.storage_cache.division_safety_constant
                        / &self.storage_cache.farm_token_supply;
                }
            }
            #[inline]
            fn get_storage_cache(&self) -> &StorageCache<M> {
                &self.storage_cache
            }
            #[inline]
            fn set_input_attributes(&mut self, attr: FarmTokenAttributes<M>) {
                self.tx_input.attributes = Some(attr);
            }
            #[inline]
            fn get_input_attributes(&self) -> Option<&FarmTokenAttributes<M>> {
                self.tx_input.attributes.as_ref()
            }
            #[inline]
            fn set_position_reward(&mut self, amount: BigUint<M>) {
                self.position_reward = amount;
            }
            #[inline]
            fn get_position_reward(&self) -> Option<&BigUint<M>> {
                Some(&self.position_reward)
            }
            #[inline]
            fn set_initial_farming_amount(&mut self, amount: BigUint<M>) {
                self.initial_farming_amount = amount;
            }
            #[inline]
            fn get_initial_farming_amount(&self) -> Option<&BigUint<M>> {
                Some(&self.initial_farming_amount)
            }
            #[inline]
            fn set_final_reward(&mut self, payment: EsdtTokenPayment<M>) {
                self.final_reward = Some(payment);
            }
            #[inline]
            fn get_final_reward(&self) -> Option<&EsdtTokenPayment<M>> {
                self.final_reward.as_ref()
            }
            #[inline]
            fn was_output_created_with_merge(&self) -> bool {
                false
            }
            #[inline]
            fn get_output_attributes(&self) -> Option<&FarmTokenAttributes<M>> {
                None
            }
            #[inline]
            fn set_output_position(&mut self, _position: FarmToken<M>, _created_with_merge: bool) {}
        }
        impl<M: ManagedTypeApi> TxInputArgs<M> for ExitFarmArgs<M> {
            #[inline]
            fn are_valid(&self) -> bool {
                true
            }
        }
        impl<M: ManagedTypeApi> TxInputPayments<M> for ExitFarmPayments<M> {
            #[inline]
            fn are_valid(&self) -> bool {
                true
            }
            #[inline]
            fn get_first(&self) -> &EsdtTokenPayment<M> {
                &self.first_payment
            }
            #[inline]
            fn get_additional(&self) -> Option<&ManagedVec<M, EsdtTokenPayment<M>>> {
                None
            }
        }
        impl<M: ManagedTypeApi> ExitFarmPayments<M> {}
        impl<M: ManagedTypeApi> TxInput<M> for ExitFarmTxInput<M> {
            #[inline]
            fn get_args(&self) -> &dyn TxInputArgs<M> {
                &self.args
            }
            #[inline]
            fn get_payments(&self) -> &dyn TxInputPayments<M> {
                &self.payments
            }
            #[inline]
            fn is_valid(&self) -> bool {
                true
            }
        }
        impl<M: ManagedTypeApi> ExitFarmTxInput<M> {}
        impl<M: ManagedTypeApi> ExitFarmContext<M> {
            #[inline]
            pub fn is_accepted_payment(&self) -> bool {
                self.tx_input.payments.first_payment.token_identifier
                    == self.storage_cache.farm_token_id
                    && self.tx_input.payments.first_payment.token_nonce != 0
                    && self.tx_input.payments.first_payment.amount != 0u64
            }
            #[inline]
            pub fn decrease_reward_reserve(&self) {
                self.storage_cache.reward_reserve -= &self.position_reward
            }
            #[inline]
            pub fn calculate_initial_farming_amount(&self) {
                self.storage_cache.reward_reserve -= &self.position_reward
            }
            #[inline]
            pub fn increase_position_reward(&mut self, amount: &BigUint<M>) {
                self.position_reward += amount;
            }
            #[inline]
            pub fn decrease_farming_token_amount(&mut self, amount: &BigUint<M>) {
                self.initial_farming_amount -= amount;
            }
        }
    }
}
pub mod ctx_events {
    use crate::contexts::{
        base::Context, claim_rewards::ClaimRewardsContext,
        compound_rewards::CompoundRewardsContext, enter_farm::EnterFarmContext,
        exit_farm::ExitFarmContext,
    };
    use common_structs::FarmTokenAttributes;
    use core::ops::{
        Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div,
        DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
    };
    use elrond_wasm::{
        api::{
            BigIntApi, BlockchainApi, CallValueApi, CryptoApi, EllipticCurveApi, ErrorApi, LogApi,
            ManagedTypeApi, PrintApi, SendApi,
        },
        arrayvec::ArrayVec,
        contract_base::{ContractBase, ProxyObjBase},
        elrond_codec::{DecodeError, NestedDecode, NestedEncode, TopDecode},
        err_msg,
        esdt::*,
        io::*,
        non_zero_usize,
        non_zero_util::*,
        only_owner, require, sc_error,
        storage::mappers::*,
        types::{
            SCResult::{Err, Ok},
            *,
        },
        Box, Vec,
    };
    use elrond_wasm::{
        derive::{ManagedVecItem, TypeAbi},
        elrond_codec,
        elrond_codec::elrond_codec_derive::{
            NestedDecode, NestedEncode, TopDecode, TopDecodeOrDefault, TopEncode,
            TopEncodeOrDefault,
        },
    };
    pub struct EnterFarmEvent<M: ManagedTypeApi> {
        caller: ManagedAddress<M>,
        farming_token_id: TokenIdentifier<M>,
        farming_token_amount: BigUint<M>,
        farm_token_id: TokenIdentifier<M>,
        farm_token_nonce: u64,
        farm_token_amount: BigUint<M>,
        farm_supply: BigUint<M>,
        reward_token_id: TokenIdentifier<M>,
        reward_token_reserve: BigUint<M>,
        farm_attributes: FarmTokenAttributes<M>,
        created_with_merge: bool,
        block: u64,
        epoch: u64,
        timestamp: u64,
    }
    impl<M: ManagedTypeApi> elrond_codec::TopEncode for EnterFarmEvent<M> {
        fn top_encode<O: elrond_codec::TopEncodeOutput>(
            &self,
            output: O,
        ) -> core::result::Result<(), elrond_codec::EncodeError> {
            let mut buffer = output.start_nested_encode();
            let dest = &mut buffer;
            elrond_codec::NestedEncode::dep_encode(&self.caller, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farming_token_id, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farming_token_amount, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farm_token_id, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farm_token_nonce, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farm_token_amount, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farm_supply, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_token_id, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_token_reserve, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farm_attributes, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.created_with_merge, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.block, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.epoch, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.timestamp, dest)?;
            output.finalize_nested_encode(buffer);
            core::result::Result::Ok(())
        }
        fn top_encode_or_exit<O: elrond_codec::TopEncodeOutput, ExitCtx: Clone>(
            &self,
            output: O,
            c: ExitCtx,
            exit: fn(ExitCtx, elrond_codec::EncodeError) -> !,
        ) {
            let mut buffer = output.start_nested_encode();
            let dest = &mut buffer;
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.caller, dest, c.clone(), exit);
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farming_token_id,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farming_token_amount,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farm_token_id,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farm_token_nonce,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farm_token_amount,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farm_supply,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_token_id,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_token_reserve,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farm_attributes,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.created_with_merge,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.block, dest, c.clone(), exit);
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.epoch, dest, c.clone(), exit);
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.timestamp, dest, c.clone(), exit);
            output.finalize_nested_encode(buffer);
        }
    }
    pub struct ExitFarmEvent<M: ManagedTypeApi> {
        caller: ManagedAddress<M>,
        farming_token_id: TokenIdentifier<M>,
        farming_token_amount: BigUint<M>,
        farm_token_id: TokenIdentifier<M>,
        farm_token_nonce: u64,
        farm_token_amount: BigUint<M>,
        farm_supply: BigUint<M>,
        reward_token_id: TokenIdentifier<M>,
        reward_token_nonce: u64,
        reward_token_amount: BigUint<M>,
        reward_reserve: BigUint<M>,
        farm_attributes: FarmTokenAttributes<M>,
        block: u64,
        epoch: u64,
        timestamp: u64,
    }
    impl<M: ManagedTypeApi> elrond_codec::TopEncode for ExitFarmEvent<M> {
        fn top_encode<O: elrond_codec::TopEncodeOutput>(
            &self,
            output: O,
        ) -> core::result::Result<(), elrond_codec::EncodeError> {
            let mut buffer = output.start_nested_encode();
            let dest = &mut buffer;
            elrond_codec::NestedEncode::dep_encode(&self.caller, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farming_token_id, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farming_token_amount, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farm_token_id, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farm_token_nonce, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farm_token_amount, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farm_supply, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_token_id, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_token_nonce, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_token_amount, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_reserve, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farm_attributes, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.block, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.epoch, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.timestamp, dest)?;
            output.finalize_nested_encode(buffer);
            core::result::Result::Ok(())
        }
        fn top_encode_or_exit<O: elrond_codec::TopEncodeOutput, ExitCtx: Clone>(
            &self,
            output: O,
            c: ExitCtx,
            exit: fn(ExitCtx, elrond_codec::EncodeError) -> !,
        ) {
            let mut buffer = output.start_nested_encode();
            let dest = &mut buffer;
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.caller, dest, c.clone(), exit);
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farming_token_id,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farming_token_amount,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farm_token_id,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farm_token_nonce,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farm_token_amount,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farm_supply,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_token_id,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_token_nonce,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_token_amount,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_reserve,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farm_attributes,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.block, dest, c.clone(), exit);
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.epoch, dest, c.clone(), exit);
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.timestamp, dest, c.clone(), exit);
            output.finalize_nested_encode(buffer);
        }
    }
    pub struct ClaimRewardsEvent<M: ManagedTypeApi> {
        caller: ManagedAddress<M>,
        old_farm_token_id: TokenIdentifier<M>,
        old_farm_token_nonce: u64,
        old_farm_token_amount: BigUint<M>,
        new_farm_token_id: TokenIdentifier<M>,
        new_farm_token_nonce: u64,
        new_farm_token_amount: BigUint<M>,
        farm_supply: BigUint<M>,
        reward_token_id: TokenIdentifier<M>,
        reward_token_nonce: u64,
        reward_token_amount: BigUint<M>,
        reward_reserve: BigUint<M>,
        old_farm_attributes: FarmTokenAttributes<M>,
        new_farm_attributes: FarmTokenAttributes<M>,
        created_with_merge: bool,
        block: u64,
        epoch: u64,
        timestamp: u64,
    }
    impl<M: ManagedTypeApi> elrond_codec::TopEncode for ClaimRewardsEvent<M> {
        fn top_encode<O: elrond_codec::TopEncodeOutput>(
            &self,
            output: O,
        ) -> core::result::Result<(), elrond_codec::EncodeError> {
            let mut buffer = output.start_nested_encode();
            let dest = &mut buffer;
            elrond_codec::NestedEncode::dep_encode(&self.caller, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.old_farm_token_id, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.old_farm_token_nonce, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.old_farm_token_amount, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.new_farm_token_id, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.new_farm_token_nonce, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.new_farm_token_amount, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farm_supply, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_token_id, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_token_nonce, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_token_amount, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_reserve, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.old_farm_attributes, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.new_farm_attributes, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.created_with_merge, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.block, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.epoch, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.timestamp, dest)?;
            output.finalize_nested_encode(buffer);
            core::result::Result::Ok(())
        }
        fn top_encode_or_exit<O: elrond_codec::TopEncodeOutput, ExitCtx: Clone>(
            &self,
            output: O,
            c: ExitCtx,
            exit: fn(ExitCtx, elrond_codec::EncodeError) -> !,
        ) {
            let mut buffer = output.start_nested_encode();
            let dest = &mut buffer;
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.caller, dest, c.clone(), exit);
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.old_farm_token_id,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.old_farm_token_nonce,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.old_farm_token_amount,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.new_farm_token_id,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.new_farm_token_nonce,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.new_farm_token_amount,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farm_supply,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_token_id,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_token_nonce,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_token_amount,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_reserve,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.old_farm_attributes,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.new_farm_attributes,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.created_with_merge,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.block, dest, c.clone(), exit);
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.epoch, dest, c.clone(), exit);
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.timestamp, dest, c.clone(), exit);
            output.finalize_nested_encode(buffer);
        }
    }
    pub struct CompoundRewardsEvent<M: ManagedTypeApi> {
        caller: ManagedAddress<M>,
        old_farm_token_id: TokenIdentifier<M>,
        old_farm_token_nonce: u64,
        old_farm_token_amount: BigUint<M>,
        new_farm_token_id: TokenIdentifier<M>,
        new_farm_token_nonce: u64,
        new_farm_token_amount: BigUint<M>,
        farm_supply: BigUint<M>,
        reward_token_id: TokenIdentifier<M>,
        reward_token_nonce: u64,
        reward_token_amount: BigUint<M>,
        reward_reserve: BigUint<M>,
        old_farm_attributes: FarmTokenAttributes<M>,
        new_farm_attributes: FarmTokenAttributes<M>,
        created_with_merge: bool,
        block: u64,
        epoch: u64,
        timestamp: u64,
    }
    impl<M: ManagedTypeApi> elrond_codec::TopEncode for CompoundRewardsEvent<M> {
        fn top_encode<O: elrond_codec::TopEncodeOutput>(
            &self,
            output: O,
        ) -> core::result::Result<(), elrond_codec::EncodeError> {
            let mut buffer = output.start_nested_encode();
            let dest = &mut buffer;
            elrond_codec::NestedEncode::dep_encode(&self.caller, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.old_farm_token_id, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.old_farm_token_nonce, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.old_farm_token_amount, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.new_farm_token_id, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.new_farm_token_nonce, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.new_farm_token_amount, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.farm_supply, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_token_id, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_token_nonce, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_token_amount, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.reward_reserve, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.old_farm_attributes, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.new_farm_attributes, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.created_with_merge, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.block, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.epoch, dest)?;
            elrond_codec::NestedEncode::dep_encode(&self.timestamp, dest)?;
            output.finalize_nested_encode(buffer);
            core::result::Result::Ok(())
        }
        fn top_encode_or_exit<O: elrond_codec::TopEncodeOutput, ExitCtx: Clone>(
            &self,
            output: O,
            c: ExitCtx,
            exit: fn(ExitCtx, elrond_codec::EncodeError) -> !,
        ) {
            let mut buffer = output.start_nested_encode();
            let dest = &mut buffer;
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.caller, dest, c.clone(), exit);
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.old_farm_token_id,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.old_farm_token_nonce,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.old_farm_token_amount,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.new_farm_token_id,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.new_farm_token_nonce,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.new_farm_token_amount,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.farm_supply,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_token_id,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_token_nonce,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_token_amount,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.reward_reserve,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.old_farm_attributes,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.new_farm_attributes,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(
                &self.created_with_merge,
                dest,
                c.clone(),
                exit,
            );
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.block, dest, c.clone(), exit);
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.epoch, dest, c.clone(), exit);
            elrond_codec::NestedEncode::dep_encode_or_exit(&self.timestamp, dest, c.clone(), exit);
            output.finalize_nested_encode(buffer);
        }
    }
    pub trait ContextEventsModule: elrond_wasm::contract_base::ContractBase + Sized {
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn emit_enter_farm_event_context(&self, ctx: &EnterFarmContext<Self::Api>) {
            let output = ctx.get_output_payments().get(0).unwrap();
            self.enter_farm_event(
                ctx.get_caller(),
                ctx.get_farm_token_id(),
                ctx.get_block_epoch(),
                &EnterFarmEvent {
                    caller: ctx.get_caller().clone(),
                    farming_token_id: ctx.get_farming_token_id().clone(),
                    farming_token_amount: ctx
                        .get_tx_input()
                        .get_payments()
                        .get_first()
                        .amount
                        .clone(),
                    farm_token_id: ctx.get_farm_token_id().clone(),
                    farm_token_nonce: output.token_nonce,
                    farm_token_amount: output.amount,
                    farm_supply: ctx.get_farm_token_supply().clone(),
                    reward_token_id: ctx.get_reward_token_id().clone(),
                    reward_token_reserve: ctx.get_reward_reserve().clone(),
                    farm_attributes: ctx.get_output_attributes().unwrap().clone(),
                    created_with_merge: ctx.was_output_created_with_merge(),
                    block: ctx.get_block_nonce(),
                    epoch: ctx.get_block_epoch(),
                    timestamp: self.blockchain().get_block_timestamp(),
                },
            )
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn emit_exit_farm_event_context(&self, ctx: &ExitFarmContext<Self::Api>) {
            let first_pay = ctx.get_tx_input().get_payments().get_first();
            let reward = ctx.get_final_reward().unwrap();
            self.exit_farm_event(
                ctx.get_caller(),
                ctx.get_farm_token_id(),
                ctx.get_block_epoch(),
                &ExitFarmEvent {
                    caller: ctx.get_caller().clone(),
                    farming_token_id: ctx.get_farming_token_id().clone(),
                    farming_token_amount: ctx.get_initial_farming_amount().unwrap().clone(),
                    farm_token_id: ctx.get_farm_token_id().clone(),
                    farm_token_nonce: first_pay.token_nonce,
                    farm_token_amount: first_pay.amount.clone(),
                    farm_supply: ctx.get_farm_token_supply().clone(),
                    reward_token_id: reward.token_identifier.clone(),
                    reward_token_nonce: reward.token_nonce,
                    reward_token_amount: reward.amount.clone(),
                    reward_reserve: ctx.get_reward_reserve().clone(),
                    farm_attributes: ctx.get_input_attributes().unwrap().clone(),
                    block: ctx.get_block_nonce(),
                    epoch: ctx.get_block_epoch(),
                    timestamp: self.blockchain().get_block_timestamp(),
                },
            )
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn emit_claim_rewards_event_context(&self, ctx: &ClaimRewardsContext<Self::Api>) {
            let first_pay = ctx.get_tx_input().get_payments().get_first();
            let reward = ctx.get_final_reward().unwrap();
            let output = ctx.get_output_payments().get(0).unwrap();
            self.claim_rewards_event(
                ctx.get_caller(),
                ctx.get_farm_token_id(),
                ctx.get_block_epoch(),
                &ClaimRewardsEvent {
                    caller: ctx.get_caller().clone(),
                    old_farm_token_id: ctx.get_farm_token_id().clone(),
                    old_farm_token_nonce: first_pay.token_nonce,
                    old_farm_token_amount: first_pay.amount,
                    new_farm_token_id: ctx.get_farm_token_id().clone(),
                    new_farm_token_nonce: output.token_nonce,
                    new_farm_token_amount: output.amount,
                    farm_supply: ctx.get_farm_token_supply().clone(),
                    reward_token_id: reward.token_identifier.clone(),
                    reward_token_nonce: reward.token_nonce,
                    reward_token_amount: reward.amount.clone(),
                    reward_reserve: ctx.get_reward_reserve().clone(),
                    old_farm_attributes: ctx.get_input_attributes().unwrap().clone(),
                    new_farm_attributes: ctx.get_output_attributes().clone(),
                    created_with_merge: ctx.was_output_created_with_merge(),
                    block: ctx.get_block_nonce(),
                    epoch: ctx.get_block_epoch(),
                    timestamp: self.blockchain().get_block_timestamp(),
                },
            )
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn emit_compound_rewards_event_context(&self, ctx: &CompoundRewardsContext<Self::Api>) {
            let first_pay = ctx.get_tx_input().get_payments().get_first();
            let reward = ctx.get_final_reward().unwrap();
            let output = ctx.get_output_payments().get(0).unwrap();
            let epoch = self.blockchain().get_block_epoch();
            self.compound_rewards_event(
                ctx.get_caller(),
                ctx.get_farm_token_id(),
                ctx.get_block_epoch(),
                &CompoundRewardsEvent {
                    caller: ctx.get_caller().clone(),
                    old_farm_token_id: ctx.get_farm_token_id().clone(),
                    old_farm_token_nonce: first_pay.token_nonce,
                    old_farm_token_amount: first_pay.amount,
                    new_farm_token_id: ctx.get_farm_token_id().clone(),
                    new_farm_token_nonce: output.token_nonce,
                    new_farm_token_amount: output.amount,
                    farm_supply: ctx.get_farm_token_supply().clone(),
                    reward_token_id: reward.token_identifier.clone(),
                    reward_token_nonce: reward.token_nonce,
                    reward_token_amount: reward.amount.clone(),
                    reward_reserve: ctx.get_reward_reserve().clone(),
                    old_farm_attributes: ctx.get_input_attributes().unwrap().clone(),
                    new_farm_attributes: ctx.get_output_attributes().clone(),
                    created_with_merge: ctx.was_output_created_with_merge(),
                    block: ctx.get_block_nonce(),
                    epoch: ctx.get_block_epoch(),
                    timestamp: self.blockchain().get_block_timestamp(),
                },
            )
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn enter_farm_event(
            &self,
            caller: &elrond_wasm::types::ManagedAddress<Self::Api>,
            farming_token: &elrond_wasm::types::TokenIdentifier<Self::Api>,
            epoch: u64,
            enter_farm_event: &EnterFarmEvent<Self::Api>,
        );
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn exit_farm_event(
            &self,
            caller: &elrond_wasm::types::ManagedAddress<Self::Api>,
            farm_token: &elrond_wasm::types::TokenIdentifier<Self::Api>,
            epoch: u64,
            exit_farm_event: &ExitFarmEvent<Self::Api>,
        );
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn claim_rewards_event(
            &self,
            caller: &elrond_wasm::types::ManagedAddress<Self::Api>,
            farm_token: &elrond_wasm::types::TokenIdentifier<Self::Api>,
            epoch: u64,
            claim_rewards_event: &ClaimRewardsEvent<Self::Api>,
        );
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn compound_rewards_event(
            &self,
            caller: &elrond_wasm::types::ManagedAddress<Self::Api>,
            farm_token: &elrond_wasm::types::TokenIdentifier<Self::Api>,
            epoch: u64,
            compound_rewards_event: &CompoundRewardsEvent<Self::Api>,
        );
    }
    pub trait AutoImpl: elrond_wasm::contract_base::ContractBase {}
    impl<C> ContextEventsModule for C
    where
        C: AutoImpl,
    {
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn enter_farm_event(
            &self,
            caller: &elrond_wasm::types::ManagedAddress<Self::Api>,
            farming_token: &elrond_wasm::types::TokenIdentifier<Self::Api>,
            epoch: u64,
            enter_farm_event: &EnterFarmEvent<Self::Api>,
        ) {
            let mut ___topic_accumulator___ = elrond_wasm::log_util::event_topic_accumulator(
                &[
                    101u8, 110u8, 116u8, 101u8, 114u8, 95u8, 102u8, 97u8, 114u8, 109u8,
                ][..],
            );
            elrond_wasm::log_util::serialize_event_topic(&mut ___topic_accumulator___, caller);
            elrond_wasm::log_util::serialize_event_topic(
                &mut ___topic_accumulator___,
                farming_token,
            );
            elrond_wasm::log_util::serialize_event_topic(&mut ___topic_accumulator___, epoch);
            let ___data_buffer___ =
                elrond_wasm::log_util::serialize_log_data(self.raw_vm_api(), enter_farm_event);
            elrond_wasm::log_util::write_log(
                self.raw_vm_api(),
                &___topic_accumulator___,
                &___data_buffer___,
            );
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn exit_farm_event(
            &self,
            caller: &elrond_wasm::types::ManagedAddress<Self::Api>,
            farm_token: &elrond_wasm::types::TokenIdentifier<Self::Api>,
            epoch: u64,
            exit_farm_event: &ExitFarmEvent<Self::Api>,
        ) {
            let mut ___topic_accumulator___ = elrond_wasm::log_util::event_topic_accumulator(
                &[101u8, 120u8, 105u8, 116u8, 95u8, 102u8, 97u8, 114u8, 109u8][..],
            );
            elrond_wasm::log_util::serialize_event_topic(&mut ___topic_accumulator___, caller);
            elrond_wasm::log_util::serialize_event_topic(&mut ___topic_accumulator___, farm_token);
            elrond_wasm::log_util::serialize_event_topic(&mut ___topic_accumulator___, epoch);
            let ___data_buffer___ =
                elrond_wasm::log_util::serialize_log_data(self.raw_vm_api(), exit_farm_event);
            elrond_wasm::log_util::write_log(
                self.raw_vm_api(),
                &___topic_accumulator___,
                &___data_buffer___,
            );
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn claim_rewards_event(
            &self,
            caller: &elrond_wasm::types::ManagedAddress<Self::Api>,
            farm_token: &elrond_wasm::types::TokenIdentifier<Self::Api>,
            epoch: u64,
            claim_rewards_event: &ClaimRewardsEvent<Self::Api>,
        ) {
            let mut ___topic_accumulator___ = elrond_wasm::log_util::event_topic_accumulator(
                &[
                    99u8, 108u8, 97u8, 105u8, 109u8, 95u8, 114u8, 101u8, 119u8, 97u8, 114u8, 100u8,
                    115u8,
                ][..],
            );
            elrond_wasm::log_util::serialize_event_topic(&mut ___topic_accumulator___, caller);
            elrond_wasm::log_util::serialize_event_topic(&mut ___topic_accumulator___, farm_token);
            elrond_wasm::log_util::serialize_event_topic(&mut ___topic_accumulator___, epoch);
            let ___data_buffer___ =
                elrond_wasm::log_util::serialize_log_data(self.raw_vm_api(), claim_rewards_event);
            elrond_wasm::log_util::write_log(
                self.raw_vm_api(),
                &___topic_accumulator___,
                &___data_buffer___,
            );
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn compound_rewards_event(
            &self,
            caller: &elrond_wasm::types::ManagedAddress<Self::Api>,
            farm_token: &elrond_wasm::types::TokenIdentifier<Self::Api>,
            epoch: u64,
            compound_rewards_event: &CompoundRewardsEvent<Self::Api>,
        ) {
            let mut ___topic_accumulator___ = elrond_wasm::log_util::event_topic_accumulator(
                &[
                    99u8, 111u8, 109u8, 112u8, 111u8, 117u8, 110u8, 100u8, 95u8, 114u8, 101u8,
                    119u8, 97u8, 114u8, 100u8, 115u8,
                ][..],
            );
            elrond_wasm::log_util::serialize_event_topic(&mut ___topic_accumulator___, caller);
            elrond_wasm::log_util::serialize_event_topic(&mut ___topic_accumulator___, farm_token);
            elrond_wasm::log_util::serialize_event_topic(&mut ___topic_accumulator___, epoch);
            let ___data_buffer___ = elrond_wasm::log_util::serialize_log_data(
                self.raw_vm_api(),
                compound_rewards_event,
            );
            elrond_wasm::log_util::write_log(
                self.raw_vm_api(),
                &___topic_accumulator___,
                &___data_buffer___,
            );
        }
    }
    pub trait EndpointWrappers:
        elrond_wasm::contract_base::ContractBase + ContextEventsModule
    {
        fn call(&self, fn_name: &[u8]) -> bool {
            if match fn_name {
                b"callBack" => {
                    self::EndpointWrappers::callback(self);
                    return true;
                }
                other => false,
            } {
                return true;
            }
            false
        }
        fn callback_selector(
            &self,
            mut ___cb_closure___: elrond_wasm::types::CallbackClosureForDeser<Self::Api>,
        ) -> elrond_wasm::types::CallbackSelectorResult<Self::Api> {
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(___cb_closure___)
        }
        fn callback(&self) {}
    }
    pub struct AbiProvider {}
    impl elrond_wasm::contract_base::ContractAbiProvider for AbiProvider {
        type Api = elrond_wasm::api::uncallable::UncallableApi;
        fn abi() -> elrond_wasm::abi::ContractAbi {
            let mut contract_abi = elrond_wasm :: abi :: ContractAbi { build_info : elrond_wasm :: abi :: BuildInfoAbi { contract_crate : elrond_wasm :: abi :: ContractCrateBuildAbi { name : "farm_with_lock" , version : "0.0.0" , } , framework : elrond_wasm :: abi :: FrameworkBuildAbi :: create () , } , docs : & [] , name : "ContextEventsModule" , constructors : Vec :: new () , endpoints : Vec :: new () , has_callback : false , type_descriptions : < elrond_wasm :: abi :: TypeDescriptionContainerImpl as elrond_wasm :: abi :: TypeDescriptionContainer > :: new () , } ;
            contract_abi
        }
    }
    pub struct ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        api: A,
    }
    impl<A> elrond_wasm::contract_base::ContractBase for ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        type Api = A;
        fn raw_vm_api(&self) -> Self::Api {
            self.api.clone()
        }
    }
    impl<A> AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> EndpointWrappers for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> elrond_wasm::contract_base::CallableContract<A> for ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        fn call(&self, fn_name: &[u8]) -> bool {
            EndpointWrappers::call(self, fn_name)
        }
        fn into_api(self: Box<Self>) -> A {
            self.api
        }
    }
    pub fn contract_obj<A>(api: A) -> ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        ContractObj { api }
    }
    #[allow(non_snake_case)]
    pub mod endpoints {
        use super::EndpointWrappers;
    }
    pub trait ProxyTrait: elrond_wasm::contract_base::ProxyObjBase + Sized {}
}
pub mod custom_config {
    use core::ops::{
        Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div,
        DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
    };
    use elrond_wasm::{
        api::{
            BigIntApi, BlockchainApi, CallValueApi, CryptoApi, EllipticCurveApi, ErrorApi, LogApi,
            ManagedTypeApi, PrintApi, SendApi,
        },
        arrayvec::ArrayVec,
        contract_base::{ContractBase, ProxyObjBase},
        elrond_codec::{DecodeError, NestedDecode, NestedEncode, TopDecode},
        err_msg,
        esdt::*,
        io::*,
        non_zero_usize,
        non_zero_util::*,
        only_owner, require, sc_error,
        storage::mappers::*,
        types::{
            SCResult::{Err, Ok},
            *,
        },
        Box, Vec,
    };
    use elrond_wasm::{
        derive::{ManagedVecItem, TypeAbi},
        elrond_codec,
        elrond_codec::elrond_codec_derive::{
            NestedDecode, NestedEncode, TopDecode, TopDecodeOrDefault, TopEncode,
            TopEncodeOrDefault,
        },
    };
    pub trait CustomConfigModule:
        elrond_wasm::contract_base::ContractBase
        + Sized
        + config::ConfigModule
        + token_send::TokenSendModule
    {
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn locked_asset_factory_address(
            &self,
        ) -> SingleValueMapper<Self::Api, elrond_wasm::types::ManagedAddress<Self::Api>>;
    }
    pub trait AutoImpl: elrond_wasm::contract_base::ContractBase {}
    impl<C> CustomConfigModule for C
    where
        C: AutoImpl + config::ConfigModule + token_send::TokenSendModule,
    {
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn locked_asset_factory_address(
            &self,
        ) -> SingleValueMapper<Self::Api, elrond_wasm::types::ManagedAddress<Self::Api>> {
            let mut ___key___ = elrond_wasm::storage::StorageKey::<Self::Api>::new(
                self.raw_vm_api(),
                &b"locked_asset_factory_address"[..],
            );
            < SingleValueMapper < Self :: Api , elrond_wasm :: types :: ManagedAddress < Self :: Api > > as elrond_wasm :: storage :: mappers :: StorageMapper < Self :: Api > > :: new (self . raw_vm_api () , ___key___)
        }
    }
    pub trait EndpointWrappers:
        elrond_wasm::contract_base::ContractBase
        + CustomConfigModule
        + config::EndpointWrappers
        + token_send::EndpointWrappers
    {
        #[inline]
        fn call_locked_asset_factory_address(&self) {
            elrond_wasm::api::CallValueApi::check_not_payable(&self.raw_vm_api());
            elrond_wasm::api::EndpointArgumentApi::check_num_arguments(&self.raw_vm_api(), 0i32);
            let result = self.locked_asset_factory_address();
            elrond_wasm::io::EndpointResult::finish(&result, self.raw_vm_api());
        }
        fn call(&self, fn_name: &[u8]) -> bool {
            if match fn_name {
                b"callBack" => {
                    self::EndpointWrappers::callback(self);
                    return true;
                }
                [103u8, 101u8, 116u8, 76u8, 111u8, 99u8, 107u8, 101u8, 100u8, 65u8, 115u8, 115u8, 101u8, 116u8, 70u8, 97u8, 99u8, 116u8, 111u8, 114u8, 121u8, 77u8, 97u8, 110u8, 97u8, 103u8, 101u8, 100u8, 65u8, 100u8, 100u8, 114u8, 101u8, 115u8, 115u8] =>
                {
                    self.call_locked_asset_factory_address();
                    true
                }
                other => false,
            } {
                return true;
            }
            if config::EndpointWrappers::call(self, fn_name) {
                return true;
            }
            if token_send::EndpointWrappers::call(self, fn_name) {
                return true;
            }
            false
        }
        fn callback_selector(
            &self,
            mut ___cb_closure___: elrond_wasm::types::CallbackClosureForDeser<Self::Api>,
        ) -> elrond_wasm::types::CallbackSelectorResult<Self::Api> {
            let mut ___call_result_loader___ = EndpointDynArgLoader::new(self.raw_vm_api());
            let ___cb_closure_matcher___ = ___cb_closure___.matcher::<32usize>();
            if ___cb_closure_matcher___.matches_empty() {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            match config::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                elrond_wasm::types::CallbackSelectorResult::Processed => {
                    return elrond_wasm::types::CallbackSelectorResult::Processed;
                }
                elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                    ___cb_closure___ = recovered_cb_closure;
                }
            }
            match token_send::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                elrond_wasm::types::CallbackSelectorResult::Processed => {
                    return elrond_wasm::types::CallbackSelectorResult::Processed;
                }
                elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                    ___cb_closure___ = recovered_cb_closure;
                }
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(___cb_closure___)
        }
        fn callback(&self) {
            if let Some(___cb_closure___) =
                elrond_wasm::types::CallbackClosureForDeser::storage_load_and_clear(
                    self.raw_vm_api(),
                )
            {
                if let elrond_wasm::types::CallbackSelectorResult::NotProcessed(_) =
                    self::EndpointWrappers::callback_selector(self, ___cb_closure___)
                {
                    elrond_wasm::api::ErrorApi::signal_error(
                        &self.raw_vm_api(),
                        err_msg::CALLBACK_BAD_FUNC,
                    );
                }
            }
        }
    }
    pub struct AbiProvider {}
    impl elrond_wasm::contract_base::ContractAbiProvider for AbiProvider {
        type Api = elrond_wasm::api::uncallable::UncallableApi;
        fn abi() -> elrond_wasm::abi::ContractAbi {
            let mut contract_abi = elrond_wasm :: abi :: ContractAbi { build_info : elrond_wasm :: abi :: BuildInfoAbi { contract_crate : elrond_wasm :: abi :: ContractCrateBuildAbi { name : "farm_with_lock" , version : "0.0.0" , } , framework : elrond_wasm :: abi :: FrameworkBuildAbi :: create () , } , docs : & [] , name : "CustomConfigModule" , constructors : Vec :: new () , endpoints : Vec :: new () , has_callback : false , type_descriptions : < elrond_wasm :: abi :: TypeDescriptionContainerImpl as elrond_wasm :: abi :: TypeDescriptionContainer > :: new () , } ;
            let mut endpoint_abi = elrond_wasm::abi::EndpointAbi {
                docs: &[],
                name: "getLockedAssetFactoryManagedAddress",
                only_owner: false,
                mutability: elrond_wasm::abi::EndpointMutabilityAbi::Readonly,
                payable_in_tokens: &[],
                inputs: Vec::new(),
                outputs: Vec::new(),
            };
            endpoint_abi . add_output :: < SingleValueMapper < Self :: Api , elrond_wasm :: types :: ManagedAddress < Self :: Api > > > (& []) ;
            contract_abi . add_type_descriptions :: < SingleValueMapper < Self :: Api , elrond_wasm :: types :: ManagedAddress < Self :: Api > > > () ;
            contract_abi.endpoints.push(endpoint_abi);
            contract_abi
        }
    }
    pub struct ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        api: A,
    }
    impl<A> elrond_wasm::contract_base::ContractBase for ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        type Api = A;
        fn raw_vm_api(&self) -> Self::Api {
            self.api.clone()
        }
    }
    impl<A> config::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> token_send::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> config::EndpointWrappers for ContractObj<A> where
        A: elrond_wasm::api::VMApi + Clone + 'static
    {
    }
    impl<A> token_send::EndpointWrappers for ContractObj<A> where
        A: elrond_wasm::api::VMApi + Clone + 'static
    {
    }
    impl<A> EndpointWrappers for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> elrond_wasm::contract_base::CallableContract<A> for ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        fn call(&self, fn_name: &[u8]) -> bool {
            EndpointWrappers::call(self, fn_name)
        }
        fn into_api(self: Box<Self>) -> A {
            self.api
        }
    }
    pub fn contract_obj<A>(api: A) -> ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        ContractObj { api }
    }
    pub use config::endpoints as __endpoints_0__;
    pub use token_send::endpoints as __endpoints_1__;
    #[allow(non_snake_case)]
    pub mod endpoints {
        use super::EndpointWrappers;
        pub use super::__endpoints_0__::*;
        pub use super::__endpoints_1__::*;
        pub fn getLockedAssetFactoryManagedAddress<A>(api: A)
        where
            A: elrond_wasm::api::VMApi + Clone + 'static,
        {
            super::contract_obj(api).call_locked_asset_factory_address();
        }
    }
    pub trait ProxyTrait:
        elrond_wasm::contract_base::ProxyObjBase
        + Sized
        + config::ProxyTrait
        + token_send::ProxyTrait
    {
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]        fn locked_asset_factory_address (self) -> elrond_wasm :: types :: ContractCall < Self :: Api , < SingleValueMapper < Self :: Api , elrond_wasm :: types :: ManagedAddress < Self :: Api > > as elrond_wasm :: io :: EndpointResult > :: DecodeAs >{
            let (___api___, ___address___) = self.into_fields();
            let mut ___contract_call___ = elrond_wasm::types::new_contract_call(
                ___api___.clone(),
                ___address___,
                &b"getLockedAssetFactoryManagedAddress"[..],
                ManagedVec::<Self::Api, EsdtTokenPayment<Self::Api>>::new(),
            );
            ___contract_call___
        }
    }
}
pub mod custom_rewards {
    use super::custom_config;
    use super::errors::*;
    use crate::assert;
    use crate::contexts::base::Context;
    use core::ops::{
        Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div,
        DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
    };
    use elrond_wasm::{
        api::{
            BigIntApi, BlockchainApi, CallValueApi, CryptoApi, EllipticCurveApi, ErrorApi, LogApi,
            ManagedTypeApi, PrintApi, SendApi,
        },
        arrayvec::ArrayVec,
        contract_base::{ContractBase, ProxyObjBase},
        elrond_codec::{DecodeError, NestedDecode, NestedEncode, TopDecode},
        err_msg,
        esdt::*,
        io::*,
        non_zero_usize,
        non_zero_util::*,
        only_owner, require, sc_error,
        storage::mappers::*,
        types::{
            SCResult::{Err, Ok},
            *,
        },
        Box, Vec,
    };
    use elrond_wasm::{
        derive::{ManagedVecItem, TypeAbi},
        elrond_codec,
        elrond_codec::elrond_codec_derive::{
            NestedDecode, NestedEncode, TopDecode, TopDecodeOrDefault, TopEncode,
            TopEncodeOrDefault,
        },
    };
    pub trait CustomRewardsModule:
        elrond_wasm::contract_base::ContractBase
        + Sized
        + config::ConfigModule
        + token_send::TokenSendModule
        + farm_token::FarmTokenModule
        + rewards::RewardsModule
        + custom_config::CustomConfigModule
    {
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn mint_per_block_rewards(
            &self,
            ctx: &mut dyn Context<Self::Api>,
        ) -> elrond_wasm::types::BigUint<Self::Api> {
            let current_block_nonce = ctx.get_block_nonce();
            let last_reward_nonce = self.last_reward_block_nonce().get();
            if current_block_nonce > last_reward_nonce {
                let to_mint =
                    self.calculate_per_block_rewards(current_block_nonce, last_reward_nonce);
                self.last_reward_block_nonce().set(&current_block_nonce);
                to_mint
            } else {
                elrond_wasm::types::BigUint::<Self::Api>::zero()
            }
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn generate_aggregated_rewards(&self, ctx: &mut dyn Context<Self::Api>) {
            let total_reward = self.mint_per_block_rewards(ctx);
            if total_reward > 0u64 {
                ctx.increase_reward_reserve(&total_reward);
                ctx.update_reward_per_share(&total_reward);
            }
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn end_produce_rewards(&self) {}
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn set_per_block_rewards(&self, per_block_amount: elrond_wasm::types::BigUint<Self::Api>) {}
    }
    pub trait AutoImpl: elrond_wasm::contract_base::ContractBase {}
    impl<C> CustomRewardsModule for C where
        C: AutoImpl
            + config::ConfigModule
            + token_send::TokenSendModule
            + farm_token::FarmTokenModule
            + rewards::RewardsModule
            + custom_config::CustomConfigModule
    {
    }
    pub trait EndpointWrappers:
        elrond_wasm::contract_base::ContractBase
        + CustomRewardsModule
        + config::EndpointWrappers
        + token_send::EndpointWrappers
        + farm_token::EndpointWrappers
        + rewards::EndpointWrappers
        + custom_config::EndpointWrappers
    {
        #[inline]
        fn call_end_produce_rewards(&self) {
            elrond_wasm::api::CallValueApi::check_not_payable(&self.raw_vm_api());
            elrond_wasm::api::EndpointArgumentApi::check_num_arguments(&self.raw_vm_api(), 0i32);
            self.end_produce_rewards();
        }
        #[inline]
        fn call_set_per_block_rewards(&self) {
            elrond_wasm::api::CallValueApi::check_not_payable(&self.raw_vm_api());
            elrond_wasm::api::EndpointArgumentApi::check_num_arguments(&self.raw_vm_api(), 1i32);
            let per_block_amount =
                elrond_wasm::load_single_arg::<Self::Api, elrond_wasm::types::BigUint<Self::Api>>(
                    self.raw_vm_api(),
                    0i32,
                    ArgId::from(&b"per_block_amount"[..]),
                );
            self.set_per_block_rewards(per_block_amount);
        }
        fn call(&self, fn_name: &[u8]) -> bool {
            if match fn_name {
                b"callBack" => {
                    self::EndpointWrappers::callback(self);
                    return true;
                }
                [101u8, 110u8, 100u8, 95u8, 112u8, 114u8, 111u8, 100u8, 117u8, 99u8, 101u8, 95u8, 114u8, 101u8, 119u8, 97u8, 114u8, 100u8, 115u8] =>
                {
                    self.call_end_produce_rewards();
                    true
                }
                [115u8, 101u8, 116u8, 80u8, 101u8, 114u8, 66u8, 108u8, 111u8, 99u8, 107u8, 82u8, 101u8, 119u8, 97u8, 114u8, 100u8, 65u8, 109u8, 111u8, 117u8, 110u8, 116u8] =>
                {
                    self.call_set_per_block_rewards();
                    true
                }
                other => false,
            } {
                return true;
            }
            if config::EndpointWrappers::call(self, fn_name) {
                return true;
            }
            if token_send::EndpointWrappers::call(self, fn_name) {
                return true;
            }
            if farm_token::EndpointWrappers::call(self, fn_name) {
                return true;
            }
            if rewards::EndpointWrappers::call(self, fn_name) {
                return true;
            }
            if custom_config::EndpointWrappers::call(self, fn_name) {
                return true;
            }
            false
        }
        fn callback_selector(
            &self,
            mut ___cb_closure___: elrond_wasm::types::CallbackClosureForDeser<Self::Api>,
        ) -> elrond_wasm::types::CallbackSelectorResult<Self::Api> {
            let mut ___call_result_loader___ = EndpointDynArgLoader::new(self.raw_vm_api());
            let ___cb_closure_matcher___ = ___cb_closure___.matcher::<32usize>();
            if ___cb_closure_matcher___.matches_empty() {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            match config::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                elrond_wasm::types::CallbackSelectorResult::Processed => {
                    return elrond_wasm::types::CallbackSelectorResult::Processed;
                }
                elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                    ___cb_closure___ = recovered_cb_closure;
                }
            }
            match token_send::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                elrond_wasm::types::CallbackSelectorResult::Processed => {
                    return elrond_wasm::types::CallbackSelectorResult::Processed;
                }
                elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                    ___cb_closure___ = recovered_cb_closure;
                }
            }
            match farm_token::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                elrond_wasm::types::CallbackSelectorResult::Processed => {
                    return elrond_wasm::types::CallbackSelectorResult::Processed;
                }
                elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                    ___cb_closure___ = recovered_cb_closure;
                }
            }
            match rewards::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                elrond_wasm::types::CallbackSelectorResult::Processed => {
                    return elrond_wasm::types::CallbackSelectorResult::Processed;
                }
                elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                    ___cb_closure___ = recovered_cb_closure;
                }
            }
            match custom_config::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                elrond_wasm::types::CallbackSelectorResult::Processed => {
                    return elrond_wasm::types::CallbackSelectorResult::Processed;
                }
                elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                    ___cb_closure___ = recovered_cb_closure;
                }
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(___cb_closure___)
        }
        fn callback(&self) {
            if let Some(___cb_closure___) =
                elrond_wasm::types::CallbackClosureForDeser::storage_load_and_clear(
                    self.raw_vm_api(),
                )
            {
                if let elrond_wasm::types::CallbackSelectorResult::NotProcessed(_) =
                    self::EndpointWrappers::callback_selector(self, ___cb_closure___)
                {
                    elrond_wasm::api::ErrorApi::signal_error(
                        &self.raw_vm_api(),
                        err_msg::CALLBACK_BAD_FUNC,
                    );
                }
            }
        }
    }
    pub struct AbiProvider {}
    impl elrond_wasm::contract_base::ContractAbiProvider for AbiProvider {
        type Api = elrond_wasm::api::uncallable::UncallableApi;
        fn abi() -> elrond_wasm::abi::ContractAbi {
            let mut contract_abi = elrond_wasm :: abi :: ContractAbi { build_info : elrond_wasm :: abi :: BuildInfoAbi { contract_crate : elrond_wasm :: abi :: ContractCrateBuildAbi { name : "farm_with_lock" , version : "0.0.0" , } , framework : elrond_wasm :: abi :: FrameworkBuildAbi :: create () , } , docs : & [] , name : "CustomRewardsModule" , constructors : Vec :: new () , endpoints : Vec :: new () , has_callback : false , type_descriptions : < elrond_wasm :: abi :: TypeDescriptionContainerImpl as elrond_wasm :: abi :: TypeDescriptionContainer > :: new () , } ;
            let mut endpoint_abi = elrond_wasm::abi::EndpointAbi {
                docs: &[],
                name: "end_produce_rewards",
                only_owner: false,
                mutability: elrond_wasm::abi::EndpointMutabilityAbi::Mutable,
                payable_in_tokens: &[],
                inputs: Vec::new(),
                outputs: Vec::new(),
            };
            contract_abi.endpoints.push(endpoint_abi);
            let mut endpoint_abi = elrond_wasm::abi::EndpointAbi {
                docs: &[],
                name: "setPerBlockRewardAmount",
                only_owner: false,
                mutability: elrond_wasm::abi::EndpointMutabilityAbi::Mutable,
                payable_in_tokens: &[],
                inputs: Vec::new(),
                outputs: Vec::new(),
            };
            endpoint_abi.add_input::<elrond_wasm::types::BigUint<Self::Api>>("per_block_amount");
            contract_abi.add_type_descriptions::<elrond_wasm::types::BigUint<Self::Api>>();
            contract_abi.endpoints.push(endpoint_abi);
            contract_abi
        }
    }
    pub struct ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        api: A,
    }
    impl<A> elrond_wasm::contract_base::ContractBase for ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        type Api = A;
        fn raw_vm_api(&self) -> Self::Api {
            self.api.clone()
        }
    }
    impl<A> config::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> token_send::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> farm_token::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> rewards::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> custom_config::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static
    {}
    impl<A> AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> config::EndpointWrappers for ContractObj<A> where
        A: elrond_wasm::api::VMApi + Clone + 'static
    {
    }
    impl<A> token_send::EndpointWrappers for ContractObj<A> where
        A: elrond_wasm::api::VMApi + Clone + 'static
    {
    }
    impl<A> farm_token::EndpointWrappers for ContractObj<A> where
        A: elrond_wasm::api::VMApi + Clone + 'static
    {
    }
    impl<A> rewards::EndpointWrappers for ContractObj<A> where
        A: elrond_wasm::api::VMApi + Clone + 'static
    {
    }
    impl<A> custom_config::EndpointWrappers for ContractObj<A> where
        A: elrond_wasm::api::VMApi + Clone + 'static
    {
    }
    impl<A> EndpointWrappers for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> elrond_wasm::contract_base::CallableContract<A> for ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        fn call(&self, fn_name: &[u8]) -> bool {
            EndpointWrappers::call(self, fn_name)
        }
        fn into_api(self: Box<Self>) -> A {
            self.api
        }
    }
    pub fn contract_obj<A>(api: A) -> ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        ContractObj { api }
    }
    pub use config::endpoints as __endpoints_0__;
    pub use custom_config::endpoints as __endpoints_4__;
    pub use farm_token::endpoints as __endpoints_2__;
    pub use rewards::endpoints as __endpoints_3__;
    pub use token_send::endpoints as __endpoints_1__;
    #[allow(non_snake_case)]
    pub mod endpoints {
        use super::EndpointWrappers;
        pub use super::__endpoints_0__::*;
        pub use super::__endpoints_1__::*;
        pub use super::__endpoints_2__::*;
        pub use super::__endpoints_3__::*;
        pub use super::__endpoints_4__::*;
        pub fn end_produce_rewards<A>(api: A)
        where
            A: elrond_wasm::api::VMApi + Clone + 'static,
        {
            super::contract_obj(api).call_end_produce_rewards();
        }
        pub fn setPerBlockRewardAmount<A>(api: A)
        where
            A: elrond_wasm::api::VMApi + Clone + 'static,
        {
            super::contract_obj(api).call_set_per_block_rewards();
        }
    }
    pub trait ProxyTrait:
        elrond_wasm::contract_base::ProxyObjBase
        + Sized
        + config::ProxyTrait
        + token_send::ProxyTrait
        + farm_token::ProxyTrait
        + rewards::ProxyTrait
        + custom_config::ProxyTrait
    {
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn end_produce_rewards(
            self,
        ) -> elrond_wasm::types::ContractCall<
            Self::Api,
            <() as elrond_wasm::io::EndpointResult>::DecodeAs,
        > {
            let (___api___, ___address___) = self.into_fields();
            let mut ___contract_call___ = elrond_wasm::types::new_contract_call(
                ___api___.clone(),
                ___address___,
                &b"end_produce_rewards"[..],
                ManagedVec::<Self::Api, EsdtTokenPayment<Self::Api>>::new(),
            );
            ___contract_call___
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn set_per_block_rewards(
            self,
            per_block_amount: elrond_wasm::types::BigUint<Self::Api>,
        ) -> elrond_wasm::types::ContractCall<
            Self::Api,
            <() as elrond_wasm::io::EndpointResult>::DecodeAs,
        > {
            let (___api___, ___address___) = self.into_fields();
            let mut ___contract_call___ = elrond_wasm::types::new_contract_call(
                ___api___.clone(),
                ___address___,
                &b"setPerBlockRewardAmount"[..],
                ManagedVec::<Self::Api, EsdtTokenPayment<Self::Api>>::new(),
            );
            ___contract_call___.push_endpoint_arg(per_block_amount);
            ___contract_call___
        }
    }
}
pub mod errors {
    pub const ERROR_NOT_ACTIVE: &[u8] = b"Not active";
    pub const ERROR_EMPTY_PAYMENTS: &[u8] = b"Empty payments";
    pub const ERROR_BAD_INPUT_TOKEN: &[u8] = b"Bad input token";
    pub const ERROR_NO_FARM_TOKEN: &[u8] = b"No farm token";
    pub const ERROR_ZERO_AMOUNT: &[u8] = b"Zero amount";
    pub const ERROR_NOT_AN_ESDT: &[u8] = b"Not a valid esdt id";
    pub const ERROR_DIFFERENT_TOKEN_IDS: &[u8] = b"Different token ids";
    pub const ERROR_SAME_TOKEN_IDS: &[u8] = b"Same token ids";
    pub const ERROR_BAD_PAYMENTS_LEN: &[u8] = b"Bad payments len";
    pub const ERROR_BAD_PAYMENTS: &[u8] = b"Bad payments";
    pub const ERROR_NOT_ENOUGH_SUPPLY: &[u8] = b"Not enough supply";
    pub const ERROR_NOT_A_FARM_TOKEN: &[u8] = b"Not a farm token";
    pub const ERROR_NO_TOKEN_TO_MERGE: &[u8] = b"No token to merge";
    pub const ERROR_PAYMENT_FAILED: &[u8] = b"Payment failed";
}
pub mod farm_token_merge {
    use super::custom_config;
    use super::errors::*;
    use crate::assert;
    use common_structs::FarmTokenAttributes;
    use core::ops::{
        Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div,
        DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
    };
    use elrond_wasm::{
        api::{
            BigIntApi, BlockchainApi, CallValueApi, CryptoApi, EllipticCurveApi, ErrorApi, LogApi,
            ManagedTypeApi, PrintApi, SendApi,
        },
        arrayvec::ArrayVec,
        contract_base::{ContractBase, ProxyObjBase},
        elrond_codec::{DecodeError, NestedDecode, NestedEncode, TopDecode},
        err_msg,
        esdt::*,
        io::*,
        non_zero_usize,
        non_zero_util::*,
        only_owner, require, sc_error,
        storage::mappers::*,
        types::{
            SCResult::{Err, Ok},
            *,
        },
        Box, Vec,
    };
    use elrond_wasm::{
        derive::{ManagedVecItem, TypeAbi},
        elrond_codec,
        elrond_codec::elrond_codec_derive::{
            NestedDecode, NestedEncode, TopDecode, TopDecodeOrDefault, TopEncode,
            TopEncodeOrDefault,
        },
    };
    use farm_token::FarmToken;
    use token_merge::ValueWeight;
    pub trait FarmTokenMergeModule:
        elrond_wasm::contract_base::ContractBase
        + Sized
        + token_send::TokenSendModule
        + farm_token::FarmTokenModule
        + custom_config::CustomConfigModule
        + config::ConfigModule
        + token_merge::TokenMergeModule
    {
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn merge_farm_tokens(
            &self,
            opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
        ) -> EsdtTokenPayment<Self::Api> {
            ::core::panicking::panic("explicit panic")
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn get_merged_farm_token_attributes(
            &self,
            payments: &ManagedVec<Self::Api, EsdtTokenPayment<Self::Api>>,
            replic: Option<&FarmToken<Self::Api>>,
        ) -> FarmTokenAttributes<Self::Api> {
            if !(!payments.is_empty() || replic.is_some()) {
                self.raw_vm_api().signal_error(ERROR_NO_TOKEN_TO_MERGE)
            };
            let mut tokens = ManagedVec::new();
            let farm_token_id = self.farm_token_id().get();
            for payment in payments.iter() {
                if !(payment.amount != 0u64) {
                    self.raw_vm_api().signal_error(ERROR_ZERO_AMOUNT)
                };
                if !(payment.token_identifier == farm_token_id) {
                    self.raw_vm_api().signal_error(ERROR_NOT_A_FARM_TOKEN)
                };
                tokens.push(FarmToken {
                    token_amount: self.create_payment(
                        &payment.token_identifier,
                        payment.token_nonce,
                        &payment.amount,
                    ),
                    attributes: self
                        .get_farm_attributes(&payment.token_identifier, payment.token_nonce)
                        .unwrap(),
                });
            }
            if let Some(r) = replic {
                tokens.push(r.clone());
            }
            if tokens.len() == 1 {
                if let Some(t) = tokens.get(0) {
                    return t.attributes;
                }
            }
            let aggregated_attributes = FarmTokenAttributes {
                reward_per_share: self.aggregated_reward_per_share(&tokens),
                entering_epoch: self.blockchain().get_block_epoch(),
                original_entering_epoch: self.aggregated_original_entering_epoch(&tokens),
                initial_farming_amount: self.aggregated_initial_farming_amount(&tokens),
                compounded_reward: self.aggregated_compounded_reward(&tokens),
                current_farm_amount: self.aggregated_current_farm_amount(&tokens),
            };
            aggregated_attributes
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn aggregated_reward_per_share(
            &self,
            tokens: &ManagedVec<Self::Api, FarmToken<Self::Api>>,
        ) -> elrond_wasm::types::BigUint<Self::Api> {
            let mut dataset = ManagedVec::new();
            tokens.iter().for_each(|x| {
                dataset.push(ValueWeight {
                    value: x.attributes.reward_per_share.clone(),
                    weight: x.token_amount.amount.clone(),
                })
            });
            self.weighted_average_ceil(dataset)
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn aggregated_initial_farming_amount(
            &self,
            tokens: &ManagedVec<Self::Api, FarmToken<Self::Api>>,
        ) -> elrond_wasm::types::BigUint<Self::Api> {
            let mut sum = elrond_wasm::types::BigUint::<Self::Api>::zero();
            for x in tokens.iter() {
                sum += &self
                    .rule_of_three_non_zero_result(
                        &x.token_amount.amount,
                        &x.attributes.current_farm_amount,
                        &x.attributes.initial_farming_amount,
                    )
                    .unwrap_or_signal_error(self.type_manager());
            }
            sum
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn aggregated_compounded_reward(
            &self,
            tokens: &ManagedVec<Self::Api, FarmToken<Self::Api>>,
        ) -> elrond_wasm::types::BigUint<Self::Api> {
            let mut sum = elrond_wasm::types::BigUint::<Self::Api>::zero();
            tokens.iter().for_each(|x| {
                sum += &self.rule_of_three(
                    &x.token_amount.amount,
                    &x.attributes.current_farm_amount,
                    &x.attributes.compounded_reward,
                )
            });
            sum
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn aggregated_current_farm_amount(
            &self,
            tokens: &ManagedVec<Self::Api, FarmToken<Self::Api>>,
        ) -> elrond_wasm::types::BigUint<Self::Api> {
            let mut aggregated_amount = elrond_wasm::types::BigUint::<Self::Api>::zero();
            tokens
                .iter()
                .for_each(|x| aggregated_amount += &x.token_amount.amount);
            aggregated_amount
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn aggregated_original_entering_epoch(
            &self,
            tokens: &ManagedVec<Self::Api, FarmToken<Self::Api>>,
        ) -> u64 {
            let mut dataset = ManagedVec::new();
            tokens.iter().for_each(|x| {
                dataset.push(ValueWeight {
                    value: elrond_wasm::types::BigUint::<Self::Api>::from(
                        x.attributes.original_entering_epoch,
                    ),
                    weight: x.token_amount.amount.clone(),
                })
            });
            let avg = self.weighted_average(dataset);
            avg.to_u64().unwrap()
        }
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn weighted_average(
            &self,
            dataset: ManagedVec<Self::Api, ValueWeight<Self::Api>>,
        ) -> elrond_wasm::types::BigUint<Self::Api> {
            let mut weight_sum = elrond_wasm::types::BigUint::<Self::Api>::zero();
            dataset
                .iter()
                .for_each(|x| weight_sum = &weight_sum + &x.weight);
            let mut elem_weight_sum = elrond_wasm::types::BigUint::<Self::Api>::zero();
            dataset
                .iter()
                .for_each(|x| elem_weight_sum += &x.value * &x.weight);
            elem_weight_sum / weight_sum
        }
    }
    pub trait AutoImpl: elrond_wasm::contract_base::ContractBase {}
    impl<C> FarmTokenMergeModule for C where
        C: AutoImpl
            + token_send::TokenSendModule
            + farm_token::FarmTokenModule
            + custom_config::CustomConfigModule
            + config::ConfigModule
            + token_merge::TokenMergeModule
    {
    }
    pub trait EndpointWrappers:
        elrond_wasm::contract_base::ContractBase
        + FarmTokenMergeModule
        + token_send::EndpointWrappers
        + farm_token::EndpointWrappers
        + custom_config::EndpointWrappers
        + config::EndpointWrappers
        + token_merge::EndpointWrappers
    {
        #[inline]
        fn call_merge_farm_tokens(&self) {
            let mut ___arg_loader = EndpointDynArgLoader::new(self.raw_vm_api());
            let opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>> =
                elrond_wasm::load_dyn_arg(
                    &mut ___arg_loader,
                    ArgId::from(&b"opt_accept_funds_func"[..]),
                );
            ___arg_loader.assert_no_more_args();
            let result = self.merge_farm_tokens(opt_accept_funds_func);
            elrond_wasm::io::EndpointResult::finish(&result, self.raw_vm_api());
        }
        fn call(&self, fn_name: &[u8]) -> bool {
            if match fn_name {
                b"callBack" => {
                    self::EndpointWrappers::callback(self);
                    return true;
                }
                [109u8, 101u8, 114u8, 103u8, 101u8, 70u8, 97u8, 114u8, 109u8, 84u8, 111u8, 107u8, 101u8, 110u8, 115u8] =>
                {
                    self.call_merge_farm_tokens();
                    true
                }
                other => false,
            } {
                return true;
            }
            if token_send::EndpointWrappers::call(self, fn_name) {
                return true;
            }
            if farm_token::EndpointWrappers::call(self, fn_name) {
                return true;
            }
            if custom_config::EndpointWrappers::call(self, fn_name) {
                return true;
            }
            if config::EndpointWrappers::call(self, fn_name) {
                return true;
            }
            if token_merge::EndpointWrappers::call(self, fn_name) {
                return true;
            }
            false
        }
        fn callback_selector(
            &self,
            mut ___cb_closure___: elrond_wasm::types::CallbackClosureForDeser<Self::Api>,
        ) -> elrond_wasm::types::CallbackSelectorResult<Self::Api> {
            let mut ___call_result_loader___ = EndpointDynArgLoader::new(self.raw_vm_api());
            let ___cb_closure_matcher___ = ___cb_closure___.matcher::<32usize>();
            if ___cb_closure_matcher___.matches_empty() {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            match token_send::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                elrond_wasm::types::CallbackSelectorResult::Processed => {
                    return elrond_wasm::types::CallbackSelectorResult::Processed;
                }
                elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                    ___cb_closure___ = recovered_cb_closure;
                }
            }
            match farm_token::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                elrond_wasm::types::CallbackSelectorResult::Processed => {
                    return elrond_wasm::types::CallbackSelectorResult::Processed;
                }
                elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                    ___cb_closure___ = recovered_cb_closure;
                }
            }
            match custom_config::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                elrond_wasm::types::CallbackSelectorResult::Processed => {
                    return elrond_wasm::types::CallbackSelectorResult::Processed;
                }
                elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                    ___cb_closure___ = recovered_cb_closure;
                }
            }
            match config::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                elrond_wasm::types::CallbackSelectorResult::Processed => {
                    return elrond_wasm::types::CallbackSelectorResult::Processed;
                }
                elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                    ___cb_closure___ = recovered_cb_closure;
                }
            }
            match token_merge::EndpointWrappers::callback_selector(self, ___cb_closure___) {
                elrond_wasm::types::CallbackSelectorResult::Processed => {
                    return elrond_wasm::types::CallbackSelectorResult::Processed;
                }
                elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                    ___cb_closure___ = recovered_cb_closure;
                }
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(___cb_closure___)
        }
        fn callback(&self) {
            if let Some(___cb_closure___) =
                elrond_wasm::types::CallbackClosureForDeser::storage_load_and_clear(
                    self.raw_vm_api(),
                )
            {
                if let elrond_wasm::types::CallbackSelectorResult::NotProcessed(_) =
                    self::EndpointWrappers::callback_selector(self, ___cb_closure___)
                {
                    elrond_wasm::api::ErrorApi::signal_error(
                        &self.raw_vm_api(),
                        err_msg::CALLBACK_BAD_FUNC,
                    );
                }
            }
        }
    }
    pub struct AbiProvider {}
    impl elrond_wasm::contract_base::ContractAbiProvider for AbiProvider {
        type Api = elrond_wasm::api::uncallable::UncallableApi;
        fn abi() -> elrond_wasm::abi::ContractAbi {
            let mut contract_abi = elrond_wasm :: abi :: ContractAbi { build_info : elrond_wasm :: abi :: BuildInfoAbi { contract_crate : elrond_wasm :: abi :: ContractCrateBuildAbi { name : "farm_with_lock" , version : "0.0.0" , } , framework : elrond_wasm :: abi :: FrameworkBuildAbi :: create () , } , docs : & [] , name : "FarmTokenMergeModule" , constructors : Vec :: new () , endpoints : Vec :: new () , has_callback : false , type_descriptions : < elrond_wasm :: abi :: TypeDescriptionContainerImpl as elrond_wasm :: abi :: TypeDescriptionContainer > :: new () , } ;
            let mut endpoint_abi = elrond_wasm::abi::EndpointAbi {
                docs: &[],
                name: "mergeFarmTokens",
                only_owner: false,
                mutability: elrond_wasm::abi::EndpointMutabilityAbi::Mutable,
                payable_in_tokens: &["*"],
                inputs: Vec::new(),
                outputs: Vec::new(),
            };
            endpoint_abi.add_input::<OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>>(
                "opt_accept_funds_func",
            );
            contract_abi
                .add_type_descriptions::<OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>>(
                );
            endpoint_abi.add_output::<EsdtTokenPayment<Self::Api>>(&[]);
            contract_abi.add_type_descriptions::<EsdtTokenPayment<Self::Api>>();
            contract_abi.endpoints.push(endpoint_abi);
            contract_abi
        }
    }
    pub struct ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        api: A,
    }
    impl<A> elrond_wasm::contract_base::ContractBase for ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        type Api = A;
        fn raw_vm_api(&self) -> Self::Api {
            self.api.clone()
        }
    }
    impl<A> token_send::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> farm_token::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> custom_config::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static
    {}
    impl<A> config::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> token_merge::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> token_send::EndpointWrappers for ContractObj<A> where
        A: elrond_wasm::api::VMApi + Clone + 'static
    {
    }
    impl<A> farm_token::EndpointWrappers for ContractObj<A> where
        A: elrond_wasm::api::VMApi + Clone + 'static
    {
    }
    impl<A> custom_config::EndpointWrappers for ContractObj<A> where
        A: elrond_wasm::api::VMApi + Clone + 'static
    {
    }
    impl<A> config::EndpointWrappers for ContractObj<A> where
        A: elrond_wasm::api::VMApi + Clone + 'static
    {
    }
    impl<A> token_merge::EndpointWrappers for ContractObj<A> where
        A: elrond_wasm::api::VMApi + Clone + 'static
    {
    }
    impl<A> EndpointWrappers for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
    impl<A> elrond_wasm::contract_base::CallableContract<A> for ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        fn call(&self, fn_name: &[u8]) -> bool {
            EndpointWrappers::call(self, fn_name)
        }
        fn into_api(self: Box<Self>) -> A {
            self.api
        }
    }
    pub fn contract_obj<A>(api: A) -> ContractObj<A>
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        ContractObj { api }
    }
    pub use config::endpoints as __endpoints_3__;
    pub use custom_config::endpoints as __endpoints_2__;
    pub use farm_token::endpoints as __endpoints_1__;
    pub use token_merge::endpoints as __endpoints_4__;
    pub use token_send::endpoints as __endpoints_0__;
    #[allow(non_snake_case)]
    pub mod endpoints {
        use super::EndpointWrappers;
        pub use super::__endpoints_0__::*;
        pub use super::__endpoints_1__::*;
        pub use super::__endpoints_2__::*;
        pub use super::__endpoints_3__::*;
        pub use super::__endpoints_4__::*;
        pub fn mergeFarmTokens<A>(api: A)
        where
            A: elrond_wasm::api::VMApi + Clone + 'static,
        {
            super::contract_obj(api).call_merge_farm_tokens();
        }
    }
    pub trait ProxyTrait:
        elrond_wasm::contract_base::ProxyObjBase
        + Sized
        + token_send::ProxyTrait
        + farm_token::ProxyTrait
        + custom_config::ProxyTrait
        + config::ProxyTrait
        + token_merge::ProxyTrait
    {
        #[allow(clippy::too_many_arguments)]
        #[allow(clippy::type_complexity)]
        fn merge_farm_tokens(
            self,
            opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
        ) -> elrond_wasm::types::ContractCall<
            Self::Api,
            <EsdtTokenPayment<Self::Api> as elrond_wasm::io::EndpointResult>::DecodeAs,
        > {
            let (___api___, ___address___) = self.into_fields();
            let mut ___contract_call___ = elrond_wasm::types::new_contract_call(
                ___api___.clone(),
                ___address___,
                &b"mergeFarmTokens"[..],
                ManagedVec::<Self::Api, EsdtTokenPayment<Self::Api>>::new(),
            );
            ___contract_call___.push_endpoint_arg(opt_accept_funds_func);
            ___contract_call___
        }
    }
}
use crate::contexts::base::*;
use common_structs::{FarmTokenAttributes, Nonce};
use config::State;
use config::{
    DEFAULT_BURN_GAS_LIMIT, DEFAULT_MINUMUM_FARMING_EPOCHS, DEFAULT_PENALTY_PERCENT,
    DEFAULT_TRANSFER_EXEC_GAS_LIMIT, MAX_PENALTY_PERCENT,
};
use contexts::exit_farm::ExitFarmContext;
use core::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div, DivAssign,
    Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
};
use elrond_wasm::{
    api::{
        BigIntApi, BlockchainApi, CallValueApi, CryptoApi, EllipticCurveApi, ErrorApi, LogApi,
        ManagedTypeApi, PrintApi, SendApi,
    },
    arrayvec::ArrayVec,
    contract_base::{ContractBase, ProxyObjBase},
    elrond_codec::{DecodeError, NestedDecode, NestedEncode, TopDecode},
    err_msg,
    esdt::*,
    io::*,
    non_zero_usize,
    non_zero_util::*,
    only_owner, require, sc_error,
    storage::mappers::*,
    types::{
        SCResult::{Err, Ok},
        *,
    },
    Box, Vec,
};
use elrond_wasm::{
    derive::{ManagedVecItem, TypeAbi},
    elrond_codec,
    elrond_codec::elrond_codec_derive::{
        NestedDecode, NestedEncode, TopDecode, TopDecodeOrDefault, TopEncode, TopEncodeOrDefault,
    },
};
use errors::*;
use farm_token::FarmToken;
type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
type CompoundRewardsResultType<BigUint> = EsdtTokenPayment<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
use factory::ProxyTrait as _;
pub trait Farm:
    elrond_wasm::contract_base::ContractBase
    + Sized
    + custom_rewards::CustomRewardsModule
    + rewards::RewardsModule
    + custom_config::CustomConfigModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + token_merge::TokenMergeModule
    + farm_token::FarmTokenModule
    + farm_token_merge::FarmTokenMergeModule
    + events::EventsModule
    + contexts::ctx_helper::CtxHelper
    + ctx_events::ContextEventsModule
{
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn init(
        &self,
        reward_token_id: elrond_wasm::types::TokenIdentifier<Self::Api>,
        farming_token_id: elrond_wasm::types::TokenIdentifier<Self::Api>,
        locked_asset_factory_address: elrond_wasm::types::ManagedAddress<Self::Api>,
        division_safety_constant: elrond_wasm::types::BigUint<Self::Api>,
        pair_contract_address: elrond_wasm::types::ManagedAddress<Self::Api>,
    ) {
        if !reward_token_id.is_esdt() {
            self.raw_vm_api().signal_error(ERROR_NOT_AN_ESDT)
        };
        if !farming_token_id.is_esdt() {
            self.raw_vm_api().signal_error(ERROR_NOT_AN_ESDT)
        };
        if !(division_safety_constant != 0u64) {
            self.raw_vm_api().signal_error(ERROR_ZERO_AMOUNT)
        };
        let farm_token = self.farm_token_id().get();
        if !(reward_token_id != farm_token) {
            self.raw_vm_api().signal_error(ERROR_SAME_TOKEN_IDS)
        };
        if !(farming_token_id != farm_token) {
            self.raw_vm_api().signal_error(ERROR_SAME_TOKEN_IDS)
        };
        self.state().set(&State::Inactive);
        self.penalty_percent()
            .set_if_empty(&DEFAULT_PENALTY_PERCENT);
        self.minimum_farming_epochs()
            .set_if_empty(&DEFAULT_MINUMUM_FARMING_EPOCHS);
        self.transfer_exec_gas_limit()
            .set_if_empty(&DEFAULT_TRANSFER_EXEC_GAS_LIMIT);
        self.burn_gas_limit().set_if_empty(&DEFAULT_BURN_GAS_LIMIT);
        self.division_safety_constant()
            .set_if_empty(&division_safety_constant);
        self.owner().set(&self.blockchain().get_caller());
        self.reward_token_id().set(&reward_token_id);
        self.farming_token_id().set(&farming_token_id);
        self.locked_asset_factory_address()
            .set(&locked_asset_factory_address);
        self.pair_contract_address().set(&pair_contract_address);
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn enter_farm(
        &self,
        opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
    ) -> EnterFarmResultType<Self::Api> {
        let mut context = self.new_enter_farm_context(opt_accept_funds_func);
        self.load_state(&mut context);
        if !(context.get_contract_state() == &State::Active) {
            self.raw_vm_api().signal_error(ERROR_NOT_ACTIVE)
        };
        self.load_farm_token_id(&mut context);
        if !!context.get_farm_token_id().is_empty() {
            self.raw_vm_api().signal_error(ERROR_NO_FARM_TOKEN)
        };
        self.load_farming_token_id(&mut context);
        if !context.is_accepted_payment() {
            self.raw_vm_api().signal_error(ERROR_BAD_PAYMENTS)
        };
        self.load_reward_token_id(&mut context);
        self.load_block_nonce(&mut context);
        self.load_block_epoch(&mut context);
        self.load_reward_per_share(&mut context);
        self.load_farm_token_supply(&mut context);
        self.load_division_safety_constant(&mut context);
        self.generate_aggregated_rewards(&mut context);
        let first_payment_amount = context
            .get_tx_input()
            .get_payments()
            .get_first()
            .amount
            .clone();
        let virtual_position = FarmToken {
            token_amount: self.create_payment(
                context.get_farm_token_id(),
                0,
                &first_payment_amount,
            ),
            attributes: FarmTokenAttributes {
                reward_per_share: context.get_reward_per_share().clone(),
                entering_epoch: context.get_block_epoch(),
                original_entering_epoch: context.get_block_epoch(),
                initial_farming_amount: first_payment_amount.clone(),
                compounded_reward: elrond_wasm::types::BigUint::<Self::Api>::zero(),
                current_farm_amount: first_payment_amount.clone(),
            },
        };
        let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
            &virtual_position,
            context
                .get_tx_input()
                .get_payments()
                .get_additional()
                .unwrap(),
            context.get_storage_cache(),
        );
        context.set_output_position(new_farm_token, created_with_merge);
        self.commit_changes(&context);
        self.execute_output_payments(&context);
        self.emit_enter_farm_event_context(&context);
        context
            .get_output_payments()
            .get(0)
            .as_ref()
            .unwrap()
            .clone()
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn exit_farm(
        &self,
        _payment_token_id: elrond_wasm::types::TokenIdentifier<Self::Api>,
        _token_nonce: Nonce,
        _amount: elrond_wasm::types::BigUint<Self::Api>,
        opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
    ) -> ExitFarmResultType<Self::Api> {
        let mut context = self.new_exit_farm_context(opt_accept_funds_func);
        self.load_state(&mut context);
        if !(context.get_contract_state() == &State::Active) {
            self.raw_vm_api().signal_error(ERROR_NOT_ACTIVE)
        };
        self.load_farm_token_id(&mut context);
        if !!context.get_farm_token_id().is_empty() {
            self.raw_vm_api().signal_error(ERROR_NO_FARM_TOKEN)
        };
        self.load_farming_token_id(&mut context);
        if !context.is_accepted_payment() {
            self.raw_vm_api().signal_error(ERROR_BAD_PAYMENTS)
        };
        self.load_reward_token_id(&mut context);
        self.load_block_nonce(&mut context);
        self.load_block_epoch(&mut context);
        self.load_reward_per_share(&mut context);
        self.load_farm_token_supply(&mut context);
        self.load_division_safety_constant(&mut context);
        self.generate_aggregated_rewards(&mut context);
        self.load_farm_attributes(&mut context);
        self.generate_aggregated_rewards(&mut context);
        self.calculate_reward(&mut context);
        context.decrease_reward_reserve();
        self.calculate_initial_farming_amount(&mut context);
        self.increase_reward_with_compounded_rewards(&mut context);
        self.burn_penalty(&mut context);
        self.burn_position(&context);
        self.commit_changes(&context);
        self.send_rewards(&mut context);
        self.construct_output_payments_exit(&mut context);
        self.execute_output_payments(&context);
        self.emit_exit_farm_event_context(&context);
        self.construct_and_get_result(&context)
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn claim_rewards(
        &self,
        opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
    ) -> ClaimRewardsResultType<Self::Api> {
        let context = self.new_claim_rewards_context(opt_accept_funds_func);
        self.load_state(&mut context);
        if !(context.get_contract_state() == &State::Active) {
            self.raw_vm_api().signal_error(ERROR_NOT_ACTIVE)
        };
        self.load_farm_token_id(&mut context);
        if !!context.get_farm_token_id().is_empty() {
            self.raw_vm_api().signal_error(ERROR_NO_FARM_TOKEN)
        };
        self.load_farming_token_id(&mut context);
        if !context.is_accepted_payment() {
            self.raw_vm_api().signal_error(ERROR_BAD_PAYMENTS)
        };
        self.load_reward_token_id(&mut context);
        self.load_block_nonce(&mut context);
        self.load_block_epoch(&mut context);
        self.load_reward_per_share(&mut context);
        self.load_farm_token_supply(&mut context);
        self.load_division_safety_constant(&mut context);
        self.generate_aggregated_rewards(&mut context);
        self.load_farm_attributes(&mut context);
        self.generate_aggregated_rewards(&mut context);
        self.calculate_reward(&mut context);
        context.decrease_reward_reserve();
        self.calculate_initial_farming_amount(&mut context);
        let new_compound_reward_amount = self.calculate_new_compound_reward_amount(&context);
        let virtual_position = FarmToken {
            token_amount: EsdtTokenPayment::new(
                context.get_farm_token_id(),
                0,
                context
                    .get_tx_input()
                    .get_payments()
                    .get_first()
                    .amount
                    .clone(),
            ),
            attributes: FarmTokenAttributes {
                reward_per_share: context.get_reward_per_share(),
                entering_epoch: context.get_input_attributes().unwrap().entering_epoch,
                original_entering_epoch: context
                    .get_input_attributes()
                    .unwrap()
                    .original_entering_epoch,
                initial_farming_amount: context.get_initial_farming_amount().unwrap().clone(),
                compounded_reward: new_compound_reward_amount,
                current_farm_amount: context
                    .get_tx_input()
                    .get_payments()
                    .get_first()
                    .amount
                    .clone(),
            },
        };
        let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
            &virtual_position,
            context
                .get_tx_input()
                .get_payments()
                .get_additional()
                .unwrap(),
            context.get_storage_cache(),
        );
        context.set_output_position(new_farm_token, created_with_merge);
        self.burn_position(&context);
        self.commit_changes(&context);
        self.send_rewards(&mut context);
        self.execute_output_payments(&context);
        self.emit_claim_rewards_event_context(&context);
        self.construct_and_get_result(&context)
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn compound_rewards(
        &self,
        opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
    ) -> CompoundRewardsResultType<Self::Api> {
        let mut context = self.new_compound_rewards_context(opt_accept_funds_func);
        self.load_state(&mut context);
        if !(context.get_contract_state() == &State::Active) {
            self.raw_vm_api().signal_error(ERROR_NOT_ACTIVE)
        };
        self.load_farm_token_id(&mut context);
        if !!context.get_farm_token_id().is_empty() {
            self.raw_vm_api().signal_error(ERROR_NO_FARM_TOKEN)
        };
        self.load_farming_token_id(&mut context);
        self.load_reward_token_id(&mut context);
        if !context.is_accepted_payment() {
            self.raw_vm_api().signal_error(ERROR_BAD_PAYMENTS)
        };
        if !(context.get_farming_token_id() == context.get_reward_token_id()) {
            self.raw_vm_api().signal_error(ERROR_DIFFERENT_TOKEN_IDS)
        };
        self.load_block_nonce(&mut context);
        self.load_block_epoch(&mut context);
        self.load_reward_per_share(&mut context);
        self.load_farm_token_supply(&mut context);
        self.load_division_safety_constant(&mut context);
        self.generate_aggregated_rewards(&mut context);
        self.load_farm_attributes(&mut context);
        self.generate_aggregated_rewards(&mut context);
        self.calculate_reward(&mut context);
        context.decrease_reward_reserve();
        self.calculate_initial_farming_amount(&mut context);
        self.calculate_new_compound_reward_amount(&mut context);
        let virtual_position = FarmToken {
            token_amount: EsdtTokenPayment::new(
                context.get_farm_token_id().clone(),
                0,
                &context.get_tx_input().get_payments().get_first().amount
                    + context.get_position_reward().unwrap(),
            ),
            attributes: FarmTokenAttributes {
                reward_per_share: context.get_reward_per_share().clone(),
                entering_epoch: context.get_block_epoch(),
                original_entering_epoch: self.aggregated_original_entering_epoch_on_compound(
                    context.get_farm_token_id(),
                    &context.get_tx_input().get_payments().get_first().amount,
                    context.get_input_attributes(),
                    context.get_position_reward().unwrap(),
                ),
                initial_farming_amount: context.get_initial_farming_amount(),
                compounded_reward: self.calculate_new_compound_reward_amount(&context)
                    + context.get_position_reward().unwrap(),
                current_farm_amount: &context.get_tx_input().get_payments().get_first().amount
                    + context.get_position_reward().unwrap(),
            },
        };
        let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
            &virtual_position,
            context
                .get_tx_input()
                .get_payments()
                .get_additional()
                .unwrap(),
            context.get_storage_cache(),
        );
        context.set_output_position(new_farm_token, created_with_merge);
        self.burn_position(&context);
        self.commit_changes(&context);
        self.execute_output_payments(&context);
        self.emit_compound_rewards_event_context(&context);
        context
            .get_output_payments()
            .get(0)
            .as_ref()
            .unwrap()
            .clone()
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn aggregated_original_entering_epoch_on_compound(
        &self,
        farm_token_id: &elrond_wasm::types::TokenIdentifier<Self::Api>,
        position_amount: &elrond_wasm::types::BigUint<Self::Api>,
        position_attributes: &FarmTokenAttributes<Self::Api>,
        reward_amount: &elrond_wasm::types::BigUint<Self::Api>,
    ) -> u64 {
        if reward_amount == &0 {
            return position_attributes.original_entering_epoch;
        }
        let initial_position = FarmToken {
            token_amount: self.create_payment(farm_token_id, 0, position_amount),
            attributes: position_attributes.clone(),
        };
        let mut reward_position = initial_position.clone();
        reward_position.token_amount.amount = reward_amount.clone();
        reward_position.attributes.original_entering_epoch = self.blockchain().get_block_epoch();
        let mut items = ManagedVec::new();
        items.push(initial_position);
        items.push(reward_position);
        self.aggregated_original_entering_epoch(&items)
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn burn_farming_tokens(
        &self,
        farming_token_id: &elrond_wasm::types::TokenIdentifier<Self::Api>,
        farming_amount: &elrond_wasm::types::BigUint<Self::Api>,
        reward_token_id: &elrond_wasm::types::TokenIdentifier<Self::Api>,
    ) {
        let pair_contract_address = self.pair_contract_address().get();
        if pair_contract_address.is_zero() {
            self.send()
                .esdt_local_burn(farming_token_id, 0, farming_amount);
        } else {
            let gas_limit = self.burn_gas_limit().get();
            self.pair_contract_proxy(pair_contract_address)
                .remove_liquidity_and_burn_token(
                    farming_token_id.clone(),
                    0,
                    farming_amount.clone(),
                    reward_token_id.clone(),
                )
                .with_gas_limit(gas_limit)
                .execute_on_dest_context_ignore_result();
        }
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn create_farm_tokens_by_merging(
        &self,
        virtual_position: &FarmToken<Self::Api>,
        additional_positions: &ManagedVec<Self::Api, EsdtTokenPayment<Self::Api>>,
        storage_cache: &StorageCache<Self::Api>,
    ) -> (FarmToken<Self::Api>, bool) {
        let additional_payments_len = additional_positions.len();
        let merged_attributes =
            self.get_merged_farm_token_attributes(additional_positions, Some(virtual_position));
        self.burn_farm_tokens_from_payments(additional_positions);
        let new_amount = merged_attributes.current_farm_amount.clone();
        let new_nonce = self.mint_farm_tokens(
            &storage_cache.farm_token_id,
            &new_amount,
            &merged_attributes,
        );
        let new_farm_token = FarmToken {
            token_amount: self.create_payment(&storage_cache.farm_token_id, new_nonce, &new_amount),
            attributes: merged_attributes,
        };
        let is_merged = additional_payments_len != 0;
        Ok((new_farm_token, is_merged))
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn send_back_farming_tokens(
        &self,
        farming_token_id: &elrond_wasm::types::TokenIdentifier<Self::Api>,
        farming_amount: &elrond_wasm::types::BigUint<Self::Api>,
        destination: &elrond_wasm::types::ManagedAddress<Self::Api>,
        opt_accept_funds_func: &OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
    ) {
        self.transfer_execute_custom(
            destination,
            farming_token_id,
            0,
            farming_amount,
            opt_accept_funds_func,
        )
        .unwrap_or_signal_error(self.type_manager());
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn send_rewards(&self, context: &mut dyn Context<Self::Api>) {
        if context.get_position_reward().unwrap() > &0u64 {
            let locked_asset_factory_address = self.locked_asset_factory_address().get();
            let result = self
                .locked_asset_factory(locked_asset_factory_address)
                .create_and_forward(
                    context.get_position_reward().clone(),
                    context.get_caller().clone(),
                    context.get_input_attributes().unwrap().entering_epoch,
                    context.get_opt_accept_funds_func().clone(),
                )
                .execute_on_dest_context_custom_range(|_, after| (after - 1, after));
            context.set_final_reward(result);
        } else {
            context.set_final_reward(self.create_payment(
                context.get_position_reward(),
                0,
                context.get_position_reward(),
            ));
        }
    }
    #[inline]
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn should_apply_penalty(&self, entering_epoch: u64) -> bool {
        entering_epoch + self.minimum_farming_epochs().get() as u64
            > self.blockchain().get_block_epoch()
    }
    #[inline]
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn get_penalty_amount(
        &self,
        amount: &elrond_wasm::types::BigUint<Self::Api>,
    ) -> elrond_wasm::types::BigUint<Self::Api> {
        amount * self.penalty_percent().get() / MAX_PENALTY_PERCENT
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn burn_penalty(&self, context: &mut ExitFarmContext<Self::Api>) {
        if self.should_apply_penalty(context.get_input_attributes().unwrap().entering_epoch) {
            let penalty_amount = self.get_penalty_amount(context.get_initial_farming_amount());
            if penalty_amount > 0u64 {
                self.burn_farming_tokens(
                    context.get_farming_token_id(),
                    &penalty_amount,
                    context.get_reward_token_id(),
                );
                context.decrease_farming_token_amount(&penalty_amount);
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn burn_position(&self, context: &dyn Context<Self::Api>) {
        let farm_token = context.get_tx_input().get_payments().get_first();
        self.burn_farm_tokens(
            &farm_token.token_identifier,
            farm_token.token_nonce,
            &farm_token.amount,
        );
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn calculate_new_compound_reward_amount(
        &self,
        context: &dyn Context<Self::Api>,
    ) -> elrond_wasm::types::BigUint<Self::Api> {
        self.rule_of_three(
            &context.get_tx_input().get_payments().get_first().amount,
            &context.get_input_attributes().unwrap().current_farm_amount,
            &context.get_input_attributes().unwrap().compounded_reward,
        );
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn locked_asset_factory(
        &self,
        to: elrond_wasm::types::ManagedAddress<Self::Api>,
    ) -> factory::Proxy<Self::Api>;
}
pub trait AutoImpl: elrond_wasm::contract_base::ContractBase {}
impl<C> Farm for C
where
    C: AutoImpl
        + custom_rewards::CustomRewardsModule
        + rewards::RewardsModule
        + custom_config::CustomConfigModule
        + config::ConfigModule
        + token_send::TokenSendModule
        + token_merge::TokenMergeModule
        + farm_token::FarmTokenModule
        + farm_token_merge::FarmTokenMergeModule
        + events::EventsModule
        + contexts::ctx_helper::CtxHelper
        + ctx_events::ContextEventsModule,
{
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn locked_asset_factory(
        &self,
        to: elrond_wasm::types::ManagedAddress<Self::Api>,
    ) -> factory::Proxy<Self::Api> {
        factory::Proxy::new_proxy_obj(self.raw_vm_api()).contract(to)
    }
}
pub trait EndpointWrappers:
    elrond_wasm::contract_base::ContractBase
    + Farm
    + custom_rewards::EndpointWrappers
    + rewards::EndpointWrappers
    + custom_config::EndpointWrappers
    + config::EndpointWrappers
    + token_send::EndpointWrappers
    + token_merge::EndpointWrappers
    + farm_token::EndpointWrappers
    + farm_token_merge::EndpointWrappers
    + events::EndpointWrappers
    + contexts::ctx_helper::EndpointWrappers
    + ctx_events::EndpointWrappers
{
    #[inline]
    fn call_init(&self) {
        elrond_wasm::api::CallValueApi::check_not_payable(&self.raw_vm_api());
        elrond_wasm::api::EndpointArgumentApi::check_num_arguments(&self.raw_vm_api(), 5i32);
        let reward_token_id = elrond_wasm::load_single_arg::<
            Self::Api,
            elrond_wasm::types::TokenIdentifier<Self::Api>,
        >(
            self.raw_vm_api(),
            0i32,
            ArgId::from(&b"reward_token_id"[..]),
        );
        let farming_token_id = elrond_wasm::load_single_arg::<
            Self::Api,
            elrond_wasm::types::TokenIdentifier<Self::Api>,
        >(
            self.raw_vm_api(),
            1i32,
            ArgId::from(&b"farming_token_id"[..]),
        );
        let locked_asset_factory_address = elrond_wasm::load_single_arg::<
            Self::Api,
            elrond_wasm::types::ManagedAddress<Self::Api>,
        >(
            self.raw_vm_api(),
            2i32,
            ArgId::from(&b"locked_asset_factory_address"[..]),
        );
        let division_safety_constant =
            elrond_wasm::load_single_arg::<Self::Api, elrond_wasm::types::BigUint<Self::Api>>(
                self.raw_vm_api(),
                3i32,
                ArgId::from(&b"division_safety_constant"[..]),
            );
        let pair_contract_address = elrond_wasm::load_single_arg::<
            Self::Api,
            elrond_wasm::types::ManagedAddress<Self::Api>,
        >(
            self.raw_vm_api(),
            4i32,
            ArgId::from(&b"pair_contract_address"[..]),
        );
        self.init(
            reward_token_id,
            farming_token_id,
            locked_asset_factory_address,
            division_safety_constant,
            pair_contract_address,
        );
    }
    #[inline]
    fn call_enter_farm(&self) {
        let mut ___arg_loader = EndpointDynArgLoader::new(self.raw_vm_api());
        let opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>> =
            elrond_wasm::load_dyn_arg(
                &mut ___arg_loader,
                ArgId::from(&b"opt_accept_funds_func"[..]),
            );
        ___arg_loader.assert_no_more_args();
        let result = self.enter_farm(opt_accept_funds_func);
        elrond_wasm::io::EndpointResult::finish(&result, self.raw_vm_api());
    }
    #[inline]
    fn call_exit_farm(&self) {
        let (_amount, _payment_token_id) =
            elrond_wasm::api::CallValueApi::payment_token_pair(&self.raw_vm_api());
        let _token_nonce = self.call_value().esdt_token_nonce();
        let mut ___arg_loader = EndpointDynArgLoader::new(self.raw_vm_api());
        let opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>> =
            elrond_wasm::load_dyn_arg(
                &mut ___arg_loader,
                ArgId::from(&b"opt_accept_funds_func"[..]),
            );
        ___arg_loader.assert_no_more_args();
        let result = self.exit_farm(
            _payment_token_id,
            _token_nonce,
            _amount,
            opt_accept_funds_func,
        );
        elrond_wasm::io::EndpointResult::finish(&result, self.raw_vm_api());
    }
    #[inline]
    fn call_claim_rewards(&self) {
        let mut ___arg_loader = EndpointDynArgLoader::new(self.raw_vm_api());
        let opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>> =
            elrond_wasm::load_dyn_arg(
                &mut ___arg_loader,
                ArgId::from(&b"opt_accept_funds_func"[..]),
            );
        ___arg_loader.assert_no_more_args();
        let result = self.claim_rewards(opt_accept_funds_func);
        elrond_wasm::io::EndpointResult::finish(&result, self.raw_vm_api());
    }
    #[inline]
    fn call_compound_rewards(&self) {
        let mut ___arg_loader = EndpointDynArgLoader::new(self.raw_vm_api());
        let opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>> =
            elrond_wasm::load_dyn_arg(
                &mut ___arg_loader,
                ArgId::from(&b"opt_accept_funds_func"[..]),
            );
        ___arg_loader.assert_no_more_args();
        let result = self.compound_rewards(opt_accept_funds_func);
        elrond_wasm::io::EndpointResult::finish(&result, self.raw_vm_api());
    }
    fn call(&self, fn_name: &[u8]) -> bool {
        if match fn_name {
            b"callBack" => {
                self::EndpointWrappers::callback(self);
                return true;
            }
            [105u8, 110u8, 105u8, 116u8] => {
                self.call_init();
                true
            }
            [101u8, 110u8, 116u8, 101u8, 114u8, 70u8, 97u8, 114u8, 109u8] => {
                self.call_enter_farm();
                true
            }
            [101u8, 120u8, 105u8, 116u8, 70u8, 97u8, 114u8, 109u8] => {
                self.call_exit_farm();
                true
            }
            [99u8, 108u8, 97u8, 105u8, 109u8, 82u8, 101u8, 119u8, 97u8, 114u8, 100u8, 115u8] => {
                self.call_claim_rewards();
                true
            }
            [99u8, 111u8, 109u8, 112u8, 111u8, 117u8, 110u8, 100u8, 82u8, 101u8, 119u8, 97u8, 114u8, 100u8, 115u8] =>
            {
                self.call_compound_rewards();
                true
            }
            other => false,
        } {
            return true;
        }
        if custom_rewards::EndpointWrappers::call(self, fn_name) {
            return true;
        }
        if rewards::EndpointWrappers::call(self, fn_name) {
            return true;
        }
        if custom_config::EndpointWrappers::call(self, fn_name) {
            return true;
        }
        if config::EndpointWrappers::call(self, fn_name) {
            return true;
        }
        if token_send::EndpointWrappers::call(self, fn_name) {
            return true;
        }
        if token_merge::EndpointWrappers::call(self, fn_name) {
            return true;
        }
        if farm_token::EndpointWrappers::call(self, fn_name) {
            return true;
        }
        if farm_token_merge::EndpointWrappers::call(self, fn_name) {
            return true;
        }
        if events::EndpointWrappers::call(self, fn_name) {
            return true;
        }
        if contexts::ctx_helper::EndpointWrappers::call(self, fn_name) {
            return true;
        }
        if ctx_events::EndpointWrappers::call(self, fn_name) {
            return true;
        }
        false
    }
    fn callback_selector(
        &self,
        mut ___cb_closure___: elrond_wasm::types::CallbackClosureForDeser<Self::Api>,
    ) -> elrond_wasm::types::CallbackSelectorResult<Self::Api> {
        let mut ___call_result_loader___ = EndpointDynArgLoader::new(self.raw_vm_api());
        let ___cb_closure_matcher___ = ___cb_closure___.matcher::<32usize>();
        if ___cb_closure_matcher___.matches_empty() {
            return elrond_wasm::types::CallbackSelectorResult::Processed;
        }
        match custom_rewards::EndpointWrappers::callback_selector(self, ___cb_closure___) {
            elrond_wasm::types::CallbackSelectorResult::Processed => {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                ___cb_closure___ = recovered_cb_closure;
            }
        }
        match rewards::EndpointWrappers::callback_selector(self, ___cb_closure___) {
            elrond_wasm::types::CallbackSelectorResult::Processed => {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                ___cb_closure___ = recovered_cb_closure;
            }
        }
        match custom_config::EndpointWrappers::callback_selector(self, ___cb_closure___) {
            elrond_wasm::types::CallbackSelectorResult::Processed => {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                ___cb_closure___ = recovered_cb_closure;
            }
        }
        match config::EndpointWrappers::callback_selector(self, ___cb_closure___) {
            elrond_wasm::types::CallbackSelectorResult::Processed => {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                ___cb_closure___ = recovered_cb_closure;
            }
        }
        match token_send::EndpointWrappers::callback_selector(self, ___cb_closure___) {
            elrond_wasm::types::CallbackSelectorResult::Processed => {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                ___cb_closure___ = recovered_cb_closure;
            }
        }
        match token_merge::EndpointWrappers::callback_selector(self, ___cb_closure___) {
            elrond_wasm::types::CallbackSelectorResult::Processed => {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                ___cb_closure___ = recovered_cb_closure;
            }
        }
        match farm_token::EndpointWrappers::callback_selector(self, ___cb_closure___) {
            elrond_wasm::types::CallbackSelectorResult::Processed => {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                ___cb_closure___ = recovered_cb_closure;
            }
        }
        match farm_token_merge::EndpointWrappers::callback_selector(self, ___cb_closure___) {
            elrond_wasm::types::CallbackSelectorResult::Processed => {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                ___cb_closure___ = recovered_cb_closure;
            }
        }
        match events::EndpointWrappers::callback_selector(self, ___cb_closure___) {
            elrond_wasm::types::CallbackSelectorResult::Processed => {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                ___cb_closure___ = recovered_cb_closure;
            }
        }
        match contexts::ctx_helper::EndpointWrappers::callback_selector(self, ___cb_closure___) {
            elrond_wasm::types::CallbackSelectorResult::Processed => {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                ___cb_closure___ = recovered_cb_closure;
            }
        }
        match ctx_events::EndpointWrappers::callback_selector(self, ___cb_closure___) {
            elrond_wasm::types::CallbackSelectorResult::Processed => {
                return elrond_wasm::types::CallbackSelectorResult::Processed;
            }
            elrond_wasm::types::CallbackSelectorResult::NotProcessed(recovered_cb_closure) => {
                ___cb_closure___ = recovered_cb_closure;
            }
        }
        elrond_wasm::types::CallbackSelectorResult::NotProcessed(___cb_closure___)
    }
    fn callback(&self) {
        if let Some(___cb_closure___) =
            elrond_wasm::types::CallbackClosureForDeser::storage_load_and_clear(self.raw_vm_api())
        {
            if let elrond_wasm::types::CallbackSelectorResult::NotProcessed(_) =
                self::EndpointWrappers::callback_selector(self, ___cb_closure___)
            {
                elrond_wasm::api::ErrorApi::signal_error(
                    &self.raw_vm_api(),
                    err_msg::CALLBACK_BAD_FUNC,
                );
            }
        }
    }
}
pub struct AbiProvider {}
impl elrond_wasm::contract_base::ContractAbiProvider for AbiProvider {
    type Api = elrond_wasm::api::uncallable::UncallableApi;
    fn abi() -> elrond_wasm::abi::ContractAbi {
        let mut contract_abi = elrond_wasm :: abi :: ContractAbi { build_info : elrond_wasm :: abi :: BuildInfoAbi { contract_crate : elrond_wasm :: abi :: ContractCrateBuildAbi { name : "farm_with_lock" , version : "0.0.0" , } , framework : elrond_wasm :: abi :: FrameworkBuildAbi :: create () , } , docs : & [] , name : "Farm" , constructors : Vec :: new () , endpoints : Vec :: new () , has_callback : false , type_descriptions : < elrond_wasm :: abi :: TypeDescriptionContainerImpl as elrond_wasm :: abi :: TypeDescriptionContainer > :: new () , } ;
        let mut endpoint_abi = elrond_wasm::abi::EndpointAbi {
            docs: &[],
            name: "init",
            only_owner: false,
            mutability: elrond_wasm::abi::EndpointMutabilityAbi::Mutable,
            payable_in_tokens: &[],
            inputs: Vec::new(),
            outputs: Vec::new(),
        };
        endpoint_abi.add_input::<elrond_wasm::types::TokenIdentifier<Self::Api>>("reward_token_id");
        contract_abi.add_type_descriptions::<elrond_wasm::types::TokenIdentifier<Self::Api>>();
        endpoint_abi
            .add_input::<elrond_wasm::types::TokenIdentifier<Self::Api>>("farming_token_id");
        contract_abi.add_type_descriptions::<elrond_wasm::types::TokenIdentifier<Self::Api>>();
        endpoint_abi.add_input::<elrond_wasm::types::ManagedAddress<Self::Api>>(
            "locked_asset_factory_address",
        );
        contract_abi.add_type_descriptions::<elrond_wasm::types::ManagedAddress<Self::Api>>();
        endpoint_abi
            .add_input::<elrond_wasm::types::BigUint<Self::Api>>("division_safety_constant");
        contract_abi.add_type_descriptions::<elrond_wasm::types::BigUint<Self::Api>>();
        endpoint_abi
            .add_input::<elrond_wasm::types::ManagedAddress<Self::Api>>("pair_contract_address");
        contract_abi.add_type_descriptions::<elrond_wasm::types::ManagedAddress<Self::Api>>();
        contract_abi.constructors.push(endpoint_abi);
        let mut endpoint_abi = elrond_wasm::abi::EndpointAbi {
            docs: &[],
            name: "enterFarm",
            only_owner: false,
            mutability: elrond_wasm::abi::EndpointMutabilityAbi::Mutable,
            payable_in_tokens: &["*"],
            inputs: Vec::new(),
            outputs: Vec::new(),
        };
        endpoint_abi.add_input::<OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>>(
            "opt_accept_funds_func",
        );
        contract_abi
            .add_type_descriptions::<OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>>();
        endpoint_abi.add_output::<EnterFarmResultType<Self::Api>>(&[]);
        contract_abi.add_type_descriptions::<EnterFarmResultType<Self::Api>>();
        contract_abi.endpoints.push(endpoint_abi);
        let mut endpoint_abi = elrond_wasm::abi::EndpointAbi {
            docs: &[],
            name: "exitFarm",
            only_owner: false,
            mutability: elrond_wasm::abi::EndpointMutabilityAbi::Mutable,
            payable_in_tokens: &["*"],
            inputs: Vec::new(),
            outputs: Vec::new(),
        };
        endpoint_abi.add_input::<OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>>(
            "opt_accept_funds_func",
        );
        contract_abi
            .add_type_descriptions::<OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>>();
        endpoint_abi.add_output::<ExitFarmResultType<Self::Api>>(&[]);
        contract_abi.add_type_descriptions::<ExitFarmResultType<Self::Api>>();
        contract_abi.endpoints.push(endpoint_abi);
        let mut endpoint_abi = elrond_wasm::abi::EndpointAbi {
            docs: &[],
            name: "claimRewards",
            only_owner: false,
            mutability: elrond_wasm::abi::EndpointMutabilityAbi::Mutable,
            payable_in_tokens: &["*"],
            inputs: Vec::new(),
            outputs: Vec::new(),
        };
        endpoint_abi.add_input::<OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>>(
            "opt_accept_funds_func",
        );
        contract_abi
            .add_type_descriptions::<OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>>();
        endpoint_abi.add_output::<ClaimRewardsResultType<Self::Api>>(&[]);
        contract_abi.add_type_descriptions::<ClaimRewardsResultType<Self::Api>>();
        contract_abi.endpoints.push(endpoint_abi);
        let mut endpoint_abi = elrond_wasm::abi::EndpointAbi {
            docs: &[],
            name: "compoundRewards",
            only_owner: false,
            mutability: elrond_wasm::abi::EndpointMutabilityAbi::Mutable,
            payable_in_tokens: &["*"],
            inputs: Vec::new(),
            outputs: Vec::new(),
        };
        endpoint_abi.add_input::<OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>>(
            "opt_accept_funds_func",
        );
        contract_abi
            .add_type_descriptions::<OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>>();
        endpoint_abi.add_output::<CompoundRewardsResultType<Self::Api>>(&[]);
        contract_abi.add_type_descriptions::<CompoundRewardsResultType<Self::Api>>();
        contract_abi.endpoints.push(endpoint_abi);
        contract_abi.coalesce(
            <custom_rewards::AbiProvider as elrond_wasm::contract_base::ContractAbiProvider>::abi(),
        );
        contract_abi.coalesce(
            <rewards::AbiProvider as elrond_wasm::contract_base::ContractAbiProvider>::abi(),
        );
        contract_abi.coalesce(
            <custom_config::AbiProvider as elrond_wasm::contract_base::ContractAbiProvider>::abi(),
        );
        contract_abi.coalesce(
            <config::AbiProvider as elrond_wasm::contract_base::ContractAbiProvider>::abi(),
        );
        contract_abi.coalesce(
            <token_send::AbiProvider as elrond_wasm::contract_base::ContractAbiProvider>::abi(),
        );
        contract_abi.coalesce(
            <token_merge::AbiProvider as elrond_wasm::contract_base::ContractAbiProvider>::abi(),
        );
        contract_abi.coalesce(
            <farm_token::AbiProvider as elrond_wasm::contract_base::ContractAbiProvider>::abi(),
        );
        contract_abi.coalesce(
            <farm_token_merge::AbiProvider as elrond_wasm::contract_base::ContractAbiProvider>::abi(
            ),
        );
        contract_abi.coalesce(
            <events::AbiProvider as elrond_wasm::contract_base::ContractAbiProvider>::abi(),
        );
        contract_abi . coalesce (< contexts :: ctx_helper :: AbiProvider as elrond_wasm :: contract_base :: ContractAbiProvider > :: abi ()) ;
        contract_abi.coalesce(
            <ctx_events::AbiProvider as elrond_wasm::contract_base::ContractAbiProvider>::abi(),
        );
        contract_abi
    }
}
pub struct ContractObj<A>
where
    A: elrond_wasm::api::VMApi + Clone + 'static,
{
    api: A,
}
impl<A> elrond_wasm::contract_base::ContractBase for ContractObj<A>
where
    A: elrond_wasm::api::VMApi + Clone + 'static,
{
    type Api = A;
    fn raw_vm_api(&self) -> Self::Api {
        self.api.clone()
    }
}
impl<A> custom_rewards::AutoImpl for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> rewards::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
impl<A> custom_config::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static
{}
impl<A> config::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
impl<A> token_send::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
impl<A> token_merge::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
impl<A> farm_token::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
impl<A> farm_token_merge::AutoImpl for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> events::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
impl<A> contexts::ctx_helper::AutoImpl for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> ctx_events::AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
impl<A> AutoImpl for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
impl<A> custom_rewards::EndpointWrappers for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> rewards::EndpointWrappers for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> custom_config::EndpointWrappers for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> config::EndpointWrappers for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> token_send::EndpointWrappers for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> token_merge::EndpointWrappers for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> farm_token::EndpointWrappers for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> farm_token_merge::EndpointWrappers for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> events::EndpointWrappers for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> contexts::ctx_helper::EndpointWrappers for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> ctx_events::EndpointWrappers for ContractObj<A> where
    A: elrond_wasm::api::VMApi + Clone + 'static
{
}
impl<A> EndpointWrappers for ContractObj<A> where A: elrond_wasm::api::VMApi + Clone + 'static {}
impl<A> elrond_wasm::contract_base::CallableContract<A> for ContractObj<A>
where
    A: elrond_wasm::api::VMApi + Clone + 'static,
{
    fn call(&self, fn_name: &[u8]) -> bool {
        EndpointWrappers::call(self, fn_name)
    }
    fn into_api(self: Box<Self>) -> A {
        self.api
    }
}
pub fn contract_obj<A>(api: A) -> ContractObj<A>
where
    A: elrond_wasm::api::VMApi + Clone + 'static,
{
    ContractObj { api }
}
pub use config::endpoints as __endpoints_3__;
pub use contexts::ctx_helper::endpoints as __endpoints_9__;
pub use ctx_events::endpoints as __endpoints_10__;
pub use custom_config::endpoints as __endpoints_2__;
pub use custom_rewards::endpoints as __endpoints_0__;
pub use events::endpoints as __endpoints_8__;
pub use farm_token::endpoints as __endpoints_6__;
pub use farm_token_merge::endpoints as __endpoints_7__;
pub use rewards::endpoints as __endpoints_1__;
pub use token_merge::endpoints as __endpoints_5__;
pub use token_send::endpoints as __endpoints_4__;
#[allow(non_snake_case)]
pub mod endpoints {
    use super::EndpointWrappers;
    pub use super::__endpoints_0__::*;
    pub use super::__endpoints_10__::*;
    pub use super::__endpoints_1__::*;
    pub use super::__endpoints_2__::*;
    pub use super::__endpoints_3__::*;
    pub use super::__endpoints_4__::*;
    pub use super::__endpoints_5__::*;
    pub use super::__endpoints_6__::*;
    pub use super::__endpoints_7__::*;
    pub use super::__endpoints_8__::*;
    pub use super::__endpoints_9__::*;
    pub fn init<A>(api: A)
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        super::contract_obj(api).call_init();
    }
    pub fn enterFarm<A>(api: A)
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        super::contract_obj(api).call_enter_farm();
    }
    pub fn exitFarm<A>(api: A)
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        super::contract_obj(api).call_exit_farm();
    }
    pub fn claimRewards<A>(api: A)
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        super::contract_obj(api).call_claim_rewards();
    }
    pub fn compoundRewards<A>(api: A)
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        super::contract_obj(api).call_compound_rewards();
    }
    pub fn callBack<A>(api: A)
    where
        A: elrond_wasm::api::VMApi + Clone + 'static,
    {
        super::contract_obj(api).callback();
    }
}
pub trait ProxyTrait:
    elrond_wasm::contract_base::ProxyObjBase
    + Sized
    + custom_rewards::ProxyTrait
    + rewards::ProxyTrait
    + custom_config::ProxyTrait
    + config::ProxyTrait
    + token_send::ProxyTrait
    + token_merge::ProxyTrait
    + farm_token::ProxyTrait
    + farm_token_merge::ProxyTrait
    + events::ProxyTrait
    + contexts::ctx_helper::ProxyTrait
    + ctx_events::ProxyTrait
{
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn init(
        self,
        reward_token_id: elrond_wasm::types::TokenIdentifier<Self::Api>,
        farming_token_id: elrond_wasm::types::TokenIdentifier<Self::Api>,
        locked_asset_factory_address: elrond_wasm::types::ManagedAddress<Self::Api>,
        division_safety_constant: elrond_wasm::types::BigUint<Self::Api>,
        pair_contract_address: elrond_wasm::types::ManagedAddress<Self::Api>,
    ) -> elrond_wasm::types::ContractDeploy<Self::Api> {
        let (___api___, ___address___) = self.into_fields();
        let mut ___contract_deploy___ =
            elrond_wasm::types::new_contract_deploy(___api___.clone(), ___address___);
        ___contract_deploy___.push_endpoint_arg(reward_token_id);
        ___contract_deploy___.push_endpoint_arg(farming_token_id);
        ___contract_deploy___.push_endpoint_arg(locked_asset_factory_address);
        ___contract_deploy___.push_endpoint_arg(division_safety_constant);
        ___contract_deploy___.push_endpoint_arg(pair_contract_address);
        ___contract_deploy___
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn enter_farm(
        self,
        opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
    ) -> elrond_wasm::types::ContractCall<
        Self::Api,
        <EnterFarmResultType<Self::Api> as elrond_wasm::io::EndpointResult>::DecodeAs,
    > {
        let (___api___, ___address___) = self.into_fields();
        let mut ___contract_call___ = elrond_wasm::types::new_contract_call(
            ___api___.clone(),
            ___address___,
            &b"enterFarm"[..],
            ManagedVec::<Self::Api, EsdtTokenPayment<Self::Api>>::new(),
        );
        ___contract_call___.push_endpoint_arg(opt_accept_funds_func);
        ___contract_call___
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn exit_farm(
        self,
        _payment_token_id: elrond_wasm::types::TokenIdentifier<Self::Api>,
        _token_nonce: Nonce,
        _amount: elrond_wasm::types::BigUint<Self::Api>,
        opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
    ) -> elrond_wasm::types::ContractCall<
        Self::Api,
        <ExitFarmResultType<Self::Api> as elrond_wasm::io::EndpointResult>::DecodeAs,
    > {
        let (___api___, ___address___) = self.into_fields();
        let mut ___contract_call___ = elrond_wasm::types::new_contract_call(
            ___api___.clone(),
            ___address___,
            &b"exitFarm"[..],
            ManagedVec::<Self::Api, EsdtTokenPayment<Self::Api>>::new(),
        );
        ___contract_call___ =
            ___contract_call___.add_token_transfer(_payment_token_id, _token_nonce, _amount);
        ___contract_call___.push_endpoint_arg(opt_accept_funds_func);
        ___contract_call___
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn claim_rewards(
        self,
        opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
    ) -> elrond_wasm::types::ContractCall<
        Self::Api,
        <ClaimRewardsResultType<Self::Api> as elrond_wasm::io::EndpointResult>::DecodeAs,
    > {
        let (___api___, ___address___) = self.into_fields();
        let mut ___contract_call___ = elrond_wasm::types::new_contract_call(
            ___api___.clone(),
            ___address___,
            &b"claimRewards"[..],
            ManagedVec::<Self::Api, EsdtTokenPayment<Self::Api>>::new(),
        );
        ___contract_call___.push_endpoint_arg(opt_accept_funds_func);
        ___contract_call___
    }
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::type_complexity)]
    fn compound_rewards(
        self,
        opt_accept_funds_func: OptionalArg<elrond_wasm::types::ManagedBuffer<Self::Api>>,
    ) -> elrond_wasm::types::ContractCall<
        Self::Api,
        <CompoundRewardsResultType<Self::Api> as elrond_wasm::io::EndpointResult>::DecodeAs,
    > {
        let (___api___, ___address___) = self.into_fields();
        let mut ___contract_call___ = elrond_wasm::types::new_contract_call(
            ___api___.clone(),
            ___address___,
            &b"compoundRewards"[..],
            ManagedVec::<Self::Api, EsdtTokenPayment<Self::Api>>::new(),
        );
        ___contract_call___.push_endpoint_arg(opt_accept_funds_func);
        ___contract_call___
    }
}
pub struct Proxy<A>
where
    A: elrond_wasm::api::VMApi + 'static,
{
    pub api: A,
    pub address: elrond_wasm::types::ManagedAddress<A>,
}
impl<A> elrond_wasm::contract_base::ProxyObjBase for Proxy<A>
where
    A: elrond_wasm::api::VMApi + 'static,
{
    type Api = A;
    fn new_proxy_obj(api: A) -> Self {
        let zero_address = ManagedAddress::zero();
        Proxy {
            api,
            address: zero_address,
        }
    }
    fn contract(mut self, address: ManagedAddress<Self::Api>) -> Self {
        self.address = address;
        self
    }
    #[inline]
    fn into_fields(self) -> (Self::Api, ManagedAddress<Self::Api>) {
        (self.api, self.address)
    }
}
impl<A> custom_rewards::ProxyTrait for Proxy<A> where A: elrond_wasm::api::VMApi {}
impl<A> rewards::ProxyTrait for Proxy<A> where A: elrond_wasm::api::VMApi {}
impl<A> custom_config::ProxyTrait for Proxy<A> where A: elrond_wasm::api::VMApi {}
impl<A> config::ProxyTrait for Proxy<A> where A: elrond_wasm::api::VMApi {}
impl<A> token_send::ProxyTrait for Proxy<A> where A: elrond_wasm::api::VMApi {}
impl<A> token_merge::ProxyTrait for Proxy<A> where A: elrond_wasm::api::VMApi {}
impl<A> farm_token::ProxyTrait for Proxy<A> where A: elrond_wasm::api::VMApi {}
impl<A> farm_token_merge::ProxyTrait for Proxy<A> where A: elrond_wasm::api::VMApi {}
impl<A> events::ProxyTrait for Proxy<A> where A: elrond_wasm::api::VMApi {}
impl<A> contexts::ctx_helper::ProxyTrait for Proxy<A> where A: elrond_wasm::api::VMApi {}
impl<A> ctx_events::ProxyTrait for Proxy<A> where A: elrond_wasm::api::VMApi {}
impl<A> ProxyTrait for Proxy<A> where A: elrond_wasm::api::VMApi {}
