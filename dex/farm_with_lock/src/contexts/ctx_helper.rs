elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::EnterFarmResultType;

use crate::assert;
use crate::errors::*;

use super::base::*;
use super::enter_farm::*;

#[elrond_wasm::module]
pub trait CtxHelper {
    fn new_enter_farm_context(
        &self,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> EnterFarmContext<Self::Api> {
        panic!()
    }
}
