elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm::module]
pub trait ValidationModule {
    fn assert(&self, cond: bool, message: &[u8]) {
        if !cond {
            self.raw_vm_api().signal_error(message);
        }
    }
}
