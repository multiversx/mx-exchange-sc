elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const PRICE_DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
const RECORD_BLOCKS_FREQUENCY: u64 = 600;
const RECORD_BUFFER_MAX_LEN: usize = 10_000;

type Nonce = u64;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi)]
pub struct PriceRecord<BigUint: BigUintApi> {
    first_token_price: BigUint,
    second_token_price: BigUint,
    start_block: Nonce,
    end_block: Nonce,
}

#[elrond_wasm_derive::module]
pub trait OracleModule {
    fn update_price_record(
        &self,
        first_token_reserve: &Self::BigUint,
        second_token_reserve: &Self::BigUint,
    ) {
        if first_token_reserve == &0 || second_token_reserve == &0 {
            return;
        }

        let current_block = self.blockchain().get_block_nonce();
        if self.known_current_block().get() != current_block {
            self.known_current_block().set(&current_block);
        } else {
            return;
        }

        let current_info_block = current_block - 1;
        let mut current_record = self.get_current_record();

        self.update_current_record(
            current_info_block,
            &mut current_record,
            first_token_reserve,
            second_token_reserve,
        );

        if self.should_commit_current_record(&current_record) {
            self.commit_current_record(&current_record);
            current_record.start_block = current_block;
            current_record.end_block = current_info_block;
        }

        self.current_price_record().set(&current_record);
    }

    fn should_commit_current_record(&self, current_record: &PriceRecord<Self::BigUint>) -> bool {
        current_record.end_block >= current_record.start_block + RECORD_BLOCKS_FREQUENCY
    }

    fn update_current_record(
        &self,
        current_info_block: Nonce,
        current_record: &mut PriceRecord<Self::BigUint>,
        first_token_reserve: &Self::BigUint,
        second_token_reserve: &Self::BigUint,
    ) {
        let instant_first_token_price =
            self.instant_price(second_token_reserve, first_token_reserve);
        let instant_second_token_price =
            self.instant_price(first_token_reserve, second_token_reserve);

        let instant_price_period = current_info_block - current_record.end_block;
        let old_price_period = current_record.end_block - current_record.start_block;

        let weighted_first_token_price = self.calculate_weighted_price(
            current_record.first_token_price.clone(),
            old_price_period,
            instant_first_token_price,
            instant_price_period,
        );
        let weighted_second_token_price = self.calculate_weighted_price(
            current_record.second_token_price.clone(),
            old_price_period,
            instant_second_token_price,
            instant_price_period,
        );

        current_record.first_token_price = weighted_first_token_price;
        current_record.second_token_price = weighted_second_token_price;
        current_record.end_block = current_info_block;
    }

    fn commit_current_record(&self, record: &PriceRecord<Self::BigUint>) {
        let len = self.price_records().len();
        if len < RECORD_BUFFER_MAX_LEN {
            self.price_records().push(record);
            self.price_records_head().set(&(len + 1));
        } else {
            let old_head = self.price_records_head().get();
            let new_head = (old_head + 1) % RECORD_BUFFER_MAX_LEN;
            self.price_records().set(new_head, record);
            self.price_records_head().set(&new_head);
        }
    }

    fn instant_price(
        &self,
        numerator: &Self::BigUint,
        denominator: &Self::BigUint,
    ) -> Self::BigUint {
        numerator * &Self::BigUint::from(PRICE_DIVISION_SAFETY_CONSTANT) / denominator.clone()
    }

    fn calculate_weighted_price(
        &self,
        weight_price: Self::BigUint,
        weight_price_period: u64,
        instant_price: Self::BigUint,
        instant_price_period: u64,
    ) -> Self::BigUint {
        (weight_price * Self::BigUint::from(weight_price_period)
            + instant_price * Self::BigUint::from(instant_price_period))
            / Self::BigUint::from(weight_price_period + instant_price_period)
    }

    fn get_current_record(&self) -> PriceRecord<Self::BigUint> {
        if self.current_price_record().is_empty() {
            let big_zero = Self::BigUint::zero();
            PriceRecord::<Self::BigUint> {
                first_token_price: big_zero.clone(),
                second_token_price: big_zero,
                start_block: 0,
                end_block: 0,
            }
        } else {
            self.current_price_record().get()
        }
    }

    fn circular_binary_search(&self, block: Nonce) -> Option<PriceRecord<Self::BigUint>> {
        let none = Option::None;
        let mut low = 1;
        let mut high = self.price_records().len();

        if low > high {
            return none;
        }

        while low <= high {
            let mid = (low + high) / 2;
            let mid_elem = self.price_records().get(mid);

            if self.record_contains_block(&mid_elem, block) {
                return Option::<PriceRecord<Self::BigUint>>::from(mid_elem);
            }

            let low_elem = self.price_records().get(low);
            let high_elem = self.price_records().get(high);

            #[allow(clippy::collapsible_else_if)]
            if mid_elem.start_block <= high_elem.start_block {
                if block > mid_elem.start_block && block <= high_elem.end_block {
                    low = mid + 1;
                } else {
                    high = mid - 1;
                }
            } else {
                if block >= low_elem.start_block && block < mid_elem.end_block {
                    high = mid - 1;
                } else {
                    low = mid + 1;
                }
            }
        }

        none
    }

    fn record_contains_block(&self, record: &PriceRecord<Self::BigUint>, block: Nonce) -> bool {
        record.start_block <= block && block <= record.end_block
    }

    #[view(getPriceRecordForBlock)]
    fn get_price_record_for_block(&self, block: Nonce) -> Option<PriceRecord<Self::BigUint>> {
        self.circular_binary_search(block)
    }

    #[view(getPriceRecordsBetweenRange)]
    fn get_price_records_starting_between_range(
        &self,
        start: usize,
        end: usize,
    ) -> MultiResultVec<PriceRecord<Self::BigUint>> {
        let mut result = MultiResultVec::<PriceRecord<Self::BigUint>>::new();
        let default_value_fn = || PriceRecord::<Self::BigUint> {
            first_token_price: Self::BigUint::zero(),
            second_token_price: Self::BigUint::zero(),
            start_block: 0u64,
            end_block: 0u64,
        };

        let mut current_index = start;
        while {
            result.push(
                self.price_records()
                    .get_or_else(current_index, default_value_fn),
            );
            current_index = (current_index + 1) % RECORD_BUFFER_MAX_LEN;
            current_index != end
        } {}

        result
    }

    #[view(getPriceRecordsLen)]
    fn get_price_records_len(&self) -> usize {
        self.price_records().len()
    }

    #[view(getPriceDivisionSafetyConstant)]
    fn get_price_division_safety_constant(&self) -> u64 {
        PRICE_DIVISION_SAFETY_CONSTANT
    }

    #[view(getPriceRecordsMaxLen)]
    fn get_price_record_max_len(&self) -> usize {
        RECORD_BUFFER_MAX_LEN
    }

    #[view(getPriceRecordBlockFrequency)]
    fn get_price_record_block_frequency(&self) -> u64 {
        RECORD_BLOCKS_FREQUENCY
    }

    #[view(getCurrentPriceRecord)]
    #[storage_mapper("current_price_record")]
    fn current_price_record(&self) -> SingleValueMapper<Self::Storage, PriceRecord<Self::BigUint>>;

    #[view(getPriceRecords)]
    #[storage_mapper("price_records")]
    fn price_records(&self) -> VecMapper<Self::Storage, PriceRecord<Self::BigUint>>;

    #[view(getPriceRecordsHead)]
    #[storage_mapper("price_records_head")]
    fn price_records_head(&self) -> SingleValueMapper<Self::Storage, usize>;

    #[storage_mapper("known_current_block")]
    fn known_current_block(&self) -> SingleValueMapper<Self::Storage, Nonce>;
}
