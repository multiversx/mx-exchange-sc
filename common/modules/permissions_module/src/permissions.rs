use bitflags::bitflags;
use multiversx_sc::{
    abi::TypeAbi,
    codec::{DecodeError, TopDecode, TopEncode},
};
bitflags! {
    pub struct Permissions: u32 {
        const NONE = 0;
        const OWNER = 1;
        const ADMIN = 2;
        const PAUSE = 4;
    }
}

impl TopEncode for Permissions {
    fn top_encode<O>(&self, output: O) -> Result<(), multiversx_sc::codec::EncodeError>
    where
        O: multiversx_sc::codec::TopEncodeOutput,
    {
        u32::top_encode(&self.bits(), output)
    }
}

impl TopDecode for Permissions {
    fn top_decode<I>(input: I) -> Result<Self, multiversx_sc::codec::DecodeError>
    where
        I: multiversx_sc::codec::TopDecodeInput,
    {
        let bits = u32::top_decode(input)?;
        Permissions::from_bits(bits).ok_or(DecodeError::INVALID_VALUE)
    }
}

impl TypeAbi for Permissions {
    fn type_name() -> multiversx_sc::abi::TypeName {
        core::any::type_name::<u32>().into()
    }
}
