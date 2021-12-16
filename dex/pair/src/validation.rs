elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[macro_export]
macro_rules! die {
    ($self:expr, $cond: expr, $msg: expr,) => {
        die!($self, $cond, $msg)
    };
    ($self:expr, $cond: expr, $msg: expr) => {
        if !$cond {
            $self.raw_vm_api().signal_error($msg);
        }
    };
}
