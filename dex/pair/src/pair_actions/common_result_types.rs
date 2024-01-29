multiversx_sc::imports!();

pub type AddLiquidityResultType<M> =
    MultiValue3<EsdtTokenPayment<M>, EsdtTokenPayment<M>, EsdtTokenPayment<M>>;

pub type RemoveLiquidityResultType<M> = MultiValue2<EsdtTokenPayment<M>, EsdtTokenPayment<M>>;

pub type SwapTokensFixedInputResultType<M> = EsdtTokenPayment<M>;

pub type SwapTokensFixedOutputResultType<M> = MultiValue2<EsdtTokenPayment<M>, EsdtTokenPayment<M>>;
