#![no_std]

#[macro_export]
macro_rules! assert {
    ($self:expr, $cond:expr, $msg:expr $(,)?) => {
        if !$cond {
            assert!($self, $msg)
        }
    };
    ($self:expr, $msg:expr $(,)?) => {
        $self.raw_vm_api().signal_error($msg)
    };
}
