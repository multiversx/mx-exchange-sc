multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem)]
pub struct ForcedDeployArg<M: ManagedTypeApi> {
    pub arg_nr: usize,
    pub value: ManagedBuffer<M>,
}

pub type ForcedDeployArgsType<M> = MultiValueEncoded<M, MultiValue2<usize, ManagedBuffer<M>>>;

#[multiversx_sc::module]
pub trait DeployModule: crate::storage::StorageModule {
    /// Forced deploy args contain the index of the arg, and the argument itself.
    ///
    /// They must be provided in the order expected by the deployed SCs arguments order.
    #[only_owner]
    #[endpoint(overwriteForcedDeployArgs)]
    fn overwrite_forced_deploy_args(&self, forced_deploy_args: ForcedDeployArgsType<Self::Api>) {
        let mut forced_args = ManagedVec::new();
        for forced_arg_multi in forced_deploy_args {
            let (index, arg) = forced_arg_multi.into_tuple();
            let forced_arg = ForcedDeployArg {
                arg_nr: index,
                value: arg,
            };
            forced_args.push(forced_arg);
        }

        self.forced_deploy_args().set(forced_args);
    }

    #[endpoint(deployContract)]
    fn deploy_contract(
        &self,
        token_used_by_sc: TokenIdentifier,
        args: MultiValueEncoded<ManagedBuffer>,
    ) -> ManagedAddress {
        require!(
            !self.all_used_tokens().contains(&token_used_by_sc),
            "Token already used"
        );

        let arg_buffer = self.prepare_deploy_args(args.into_vec_of_buffers());
        let deployed_sc_address = self.deploy_from_source(&arg_buffer);

        let address_id = self.address_id().insert_new(&deployed_sc_address);
        let _ = self.all_deployed_contracts().insert(address_id);
        self.address_for_token(&token_used_by_sc).set(address_id);

        let _ = self.all_used_tokens().insert(token_used_by_sc);

        deployed_sc_address
    }

    fn prepare_deploy_args(
        &self,
        provided_args: ManagedVec<ManagedBuffer>,
    ) -> ManagedArgBuffer<Self::Api> {
        let mut arg_buffer = ManagedArgBuffer::new();

        let mut forced_args = self.forced_deploy_args().get();
        let mut opt_current_forced_arg = self.get_next_forced_arg(&mut forced_args);

        let mut all_args_index = 0;
        let mut provided_args_index = 0;
        let provided_args_len = provided_args.len();
        while provided_args_index < provided_args_len {
            if let Some(current_forced_arg) = &opt_current_forced_arg {
                if current_forced_arg.arg_nr == all_args_index {
                    arg_buffer.push_arg_raw(current_forced_arg.value.clone());
                    opt_current_forced_arg = self.get_next_forced_arg(&mut forced_args);

                    all_args_index += 1;

                    continue;
                }
            }

            let provided_arg = provided_args.get(provided_args_index);
            arg_buffer.push_arg_raw((*provided_arg).clone());

            provided_args_index += 1;
            all_args_index += 1;
        }

        while let Some(current_forced_arg) = &opt_current_forced_arg {
            if current_forced_arg.arg_nr == all_args_index {
                arg_buffer.push_arg_raw(current_forced_arg.value.clone());
                opt_current_forced_arg = self.get_next_forced_arg(&mut forced_args);

                all_args_index += 1;
            } else {
                break;
            }
        }

        // All contracts have the admins list as last argument
        let caller = self.blockchain().get_caller();
        arg_buffer.push_arg_raw(caller.as_managed_buffer().clone());

        arg_buffer
    }

    fn get_next_forced_arg(
        &self,
        forced_args: &mut ManagedVec<ForcedDeployArg<Self::Api>>,
    ) -> Option<ForcedDeployArg<Self::Api>> {
        if forced_args.is_empty() {
            return None;
        }

        let arg = forced_args.get(0);
        forced_args.remove(0);

        Some(arg)
    }

    fn deploy_from_source(&self, args: &ManagedArgBuffer<Self::Api>) -> ManagedAddress {
        let template = self.template_address().get();
        let code_metadata =
            CodeMetadata::PAYABLE_BY_SC | CodeMetadata::READABLE | CodeMetadata::UPGRADEABLE;
        let gas_left = self.blockchain().get_gas_left();
        let (deployed_sc_address, _) = self.send_raw().deploy_from_source_contract(
            gas_left,
            &BigUint::zero(),
            &template,
            code_metadata,
            args,
        );

        deployed_sc_address
    }
}
