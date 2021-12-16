elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[macro_export]
macro_rules! err {
    ($self:expr, $msg:expr,) => {
        err!($self, $cond, $msg)
    };
    ($self:expr, $msg:expr) => {
        $self.raw_vm_api().signal_error($msg)
    };
}

#[macro_export]
macro_rules! kill {
    ($self:expr, $cond:expr, $msg:expr,) => {
        kill!($self, $cond, $msg)
    };
    ($self:expr, $cond:expr, $msg:expr) => {
        if !$cond {
            crate::err!($self, $msg)
        }
    };
}
