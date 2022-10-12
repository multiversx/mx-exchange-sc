use bitflags::bitflags;
use elrond_wasm::{
    abi::TypeAbi,
    elrond_codec::{DecodeError, TopDecode, TopEncode},
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
    fn top_encode<O>(&self, output: O) -> Result<(), elrond_wasm::elrond_codec::EncodeError>
    where
        O: elrond_wasm::elrond_codec::TopEncodeOutput,
    {
        u32::top_encode(&self.bits(), output)
    }
}

impl TopDecode for Permissions {
    fn top_decode<I>(input: I) -> Result<Self, elrond_wasm::elrond_codec::DecodeError>
    where
        I: elrond_wasm::elrond_codec::TopDecodeInput,
    {
        let bits = u32::top_decode(input)?;
        Permissions::from_bits(bits).ok_or(DecodeError::INVALID_VALUE)
    }
}

impl TypeAbi for Permissions {
    fn type_name() -> elrond_wasm::abi::TypeName {
        core::any::type_name::<u32>().into()
    }
}
