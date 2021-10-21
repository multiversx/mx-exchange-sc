elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const PRICE_DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
const RECORD_BLOCKS_FREQUENCY: u64 = 600;
const RECORD_BUFFER_MAX_LEN: usize = 10_000;

type Nonce = u64;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi)]
pub struct PriceRecord<M: ManagedTypeApi> {
    first_token_price: BigUint<M>,
    second_token_price: BigUint<M>,
    start_block: Nonce,
    end_block: Nonce,
}

#[elrond_wasm_derive::module]
pub trait OracleModule {
    fn update_price_record(&self, first_token_reserve: &BigUint, second_token_reserve: &BigUint) {
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
        if self.current_price_record().is_empty() {
            self.build_first_price_record(
                current_info_block,
                first_token_reserve,
                second_token_reserve,
            );
            return;
        }

        let mut current_record = self.current_price_record().get();
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

    fn build_first_price_record(
        &self,
        current_info_block: Nonce,
        first_token_reserve: &BigUint,
        second_token_reserve: &BigUint,
    ) {
        self.current_price_record().set(&PriceRecord::<Self::Api> {
            first_token_price: self.instant_price(second_token_reserve, first_token_reserve),
            second_token_price: self.instant_price(first_token_reserve, second_token_reserve),
            start_block: current_info_block,
            end_block: current_info_block,
        });
    }

    fn should_commit_current_record(&self, current_record: &PriceRecord<Self::Api>) -> bool {
        current_record.end_block >= current_record.start_block + RECORD_BLOCKS_FREQUENCY
    }

    fn update_current_record(
        &self,
        current_info_block: Nonce,
        current_record: &mut PriceRecord<Self::Api>,
        first_token_reserve: &BigUint,
        second_token_reserve: &BigUint,
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

    fn commit_current_record(&self, record: &PriceRecord<Self::Api>) {
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

    fn instant_price(&self, numerator: &BigUint, denominator: &BigUint) -> BigUint {
        &(numerator * &BigUint::from(PRICE_DIVISION_SAFETY_CONSTANT)) / denominator
    }

    fn calculate_weighted_price(
        &self,
        weight_price: BigUint,
        weight_price_period: u64,
        instant_price: BigUint,
        instant_price_period: u64,
    ) -> BigUint {
        (weight_price * BigUint::from(weight_price_period)
            + instant_price * BigUint::from(instant_price_period))
            / BigUint::from(weight_price_period + instant_price_period)
    }

    fn circular_binary_search(&self, block: Nonce) -> Option<PriceRecord<Self::Api>> {
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
                return Option::<PriceRecord<Self::Api>>::from(mid_elem);
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

    fn record_contains_block(&self, record: &PriceRecord<Self::Api>, block: Nonce) -> bool {
        record.start_block <= block && block <= record.end_block
    }

    #[view(getPriceRecordForBlock)]
    fn get_price_record_for_block(&self, block: Nonce) -> Option<PriceRecord<Self::Api>> {
        self.circular_binary_search(block)
    }

    #[view(getPriceRecordsBetweenRange)]
    fn get_price_records_starting_between_range(
        &self,
        start: usize,
        end: usize,
    ) -> MultiResultVec<PriceRecord<Self::Api>> {
        let mut result = Vec::new();

        let mut current_index = start;
        loop {
            result.push(
                self.price_records()
                    .get_or_else(current_index, || PriceRecord {
                        first_token_price: BigUint::zero(),
                        second_token_price: BigUint::zero(),
                        start_block: 0,
                        end_block: 0,
                    }),
            );
            current_index = (current_index + 1) % RECORD_BUFFER_MAX_LEN;

            if current_index == end {
                break;
            }
        }

        result.into()
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
    fn current_price_record(&self) -> SingleValueMapper<PriceRecord<Self::Api>>;

    #[view(getPriceRecords)]
    #[storage_mapper("price_records")]
    fn price_records(&self) -> VecMapper<PriceRecord<Self::Api>>;

    #[view(getPriceRecordsHead)]
    #[storage_mapper("price_records_head")]
    fn price_records_head(&self) -> SingleValueMapper<usize>;

    #[storage_mapper("known_current_block")]
    fn known_current_block(&self) -> SingleValueMapper<Nonce>;
}
