#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const DEFAULT_PRECISION: u64 = 1000;

mod reserve_node;
use reserve_node::{Nonce, NonceAmountPair, ReserveNode};

#[elrond_wasm::module]
pub trait ReserveQueueModule {
    #[endpoint]
    fn pop(
        &self,
        token_id: &TokenIdentifier,
        amount: &Self::BigUint,
    ) -> SCResult<Vec<NonceAmountPair<Self::BigUint>>> {
        let mut result = Vec::new();
        let big_zero = Self::BigUint::zero();
        let mut amount_left = amount.clone();
        let queues_orig = self.queues(token_id).get();
        let mut queues = queues_orig.clone();
        let precision = self.get_precision_or_default(token_id);

        while amount_left != big_zero && !queues.is_empty() {
            let queue_id = *queues.last().unwrap();
            let mut queue_result =
                self.pop_from_queue(token_id, &amount_left, precision, &mut queues, queue_id)?;

            let mut queue_result_sum_amounts = big_zero.clone();
            queue_result
                .iter()
                .for_each(|x| queue_result_sum_amounts += &x.amount);

            result.append(&mut queue_result);
            amount_left -= &queue_result_sum_amounts;
        }
        require!(amount_left == big_zero, "Not enough reserves");

        if queues != queues_orig {
            self.queues(token_id).set(&queues);
        }

        Ok(result)
    }

    #[endpoint]
    fn push(
        &self,
        token_id: &TokenIdentifier,
        pairs: &[NonceAmountPair<Self::BigUint>],
    ) -> SCResult<()> {
        let queues_orig = self.queues(token_id).get();
        let mut queues = queues_orig.clone();
        let precision = self.get_precision_or_default(token_id);
        let pairs_compressed = self.get_compressed_pairs(pairs);

        for pair in pairs_compressed.iter() {
            self.push_single(token_id, pair.nonce, &pair.amount, precision, &mut queues)?;
        }

        if queues != queues_orig {
            self.queues(token_id).set(&queues);
        }

        Ok(())
    }

    // Below funcs should be private

    fn pop_from_queue(
        &self,
        token_id: &TokenIdentifier,
        amount: &Self::BigUint,
        precision: u64,
        queues: &mut Vec<u64>,
        queue_id: u64,
    ) -> SCResult<Vec<NonceAmountPair<Self::BigUint>>> {
        let is_fungible = !self.reserve(token_id, 0).is_empty();

        if is_fungible {
            self.pop_from_queue_fungible(token_id, amount, precision, queues, queue_id)
        } else {
            self.pop_from_queue_non_fungible(token_id, amount, precision, queues, queue_id)
        }
    }

    fn pop_from_queue_fungible(
        &self,
        token_id: &TokenIdentifier,
        amount: &Self::BigUint,
        _precision: u64,
        _queues: &mut Vec<u64>,
        _queue_id: u64,
    ) -> SCResult<Vec<NonceAmountPair<Self::BigUint>>> {
        let reserve = self.reserve(token_id, 0).get();
        let mut result = Vec::new();

        if &reserve.amount > amount {
            self.reserve(token_id, 0).update(|x| x.amount -= amount);
            result.push(NonceAmountPair::from(0, amount.clone()));
        } else {
            self.reserve(token_id, 0).clear();
            result.push(NonceAmountPair::from(0, reserve.amount));
        }

        Ok(result)
    }

    fn pop_from_queue_non_fungible(
        &self,
        token_id: &TokenIdentifier,
        amount: &Self::BigUint,
        precision: u64,
        queues: &mut Vec<u64>,
        queue_id: u64,
    ) -> SCResult<Vec<NonceAmountPair<Self::BigUint>>> {
        let head = self.head(token_id, queue_id).get();
        require!(head != 0, "Empty reserve");

        let mut pairs = Vec::new();
        let mut current_nonce = head;
        let big_zero = Self::BigUint::zero();
        let mut amount_left = amount.clone();
        let mut current_elem: ReserveNode<Self::BigUint>;

        while current_nonce != 0 && amount_left != big_zero {
            current_elem = self.reserve(token_id, current_nonce).get();

            if current_elem.amount > amount_left {
                current_elem.amount -= &amount_left;

                pairs.push(NonceAmountPair::from(current_nonce, amount_left));

                let current_amount = &current_elem.amount;
                let current_queue_id = current_elem.queue_id;
                let new_queue_id = self.get_queue_id(current_amount, precision);

                if self.should_change_queue(current_queue_id, new_queue_id) {
                    self.reserve(token_id, current_nonce).clear();
                    self.push_single_non_fungible_new(
                        token_id,
                        current_nonce,
                        current_amount,
                        precision,
                        queues,
                        Option::Some(new_queue_id),
                    );
                    current_nonce = current_elem.next;
                } else {
                    self.reserve(token_id, current_nonce).set(&current_elem);
                }

                amount_left = big_zero.clone();
            } else {
                self.reserve(token_id, current_nonce).clear();

                let current_amount = &current_elem.amount;
                pairs.push(NonceAmountPair::from(current_nonce, current_amount.clone()));

                current_nonce = current_elem.next;
                amount_left -= current_amount;
            }
        }

        if current_nonce == 0 {
            self.head(token_id, queue_id).clear();
            self.tail(token_id, queue_id).clear();
            queues.pop();
        } else if current_nonce != head {
            self.head(token_id, queue_id).set(&current_nonce);
        }

        Ok(pairs)
    }

    fn push_single(
        &self,
        token_id: &TokenIdentifier,
        nonce: Nonce,
        amount: &Self::BigUint,
        precision: u64,
        queues: &mut Vec<u64>,
    ) -> SCResult<()> {
        let is_fungible = nonce == 0;

        if is_fungible {
            self.push_single_fungible(token_id, nonce, amount, precision, queues)
        } else {
            self.push_single_non_fungible(token_id, nonce, amount, precision, queues)
        }
    }

    fn push_single_fungible(
        &self,
        token_id: &TokenIdentifier,
        nonce: Nonce,
        amount: &Self::BigUint,
        _precision: u64,
        _queues: &mut Vec<u64>,
    ) -> SCResult<()> {
        let exists = !self.reserve(token_id, nonce).is_empty();

        if exists {
            self.reserve(token_id, nonce).update(|x| x.amount += amount);
        } else {
            self.reserve(token_id, nonce)
                .set(&ReserveNode::from(amount.clone(), 0));
        }

        Ok(())
    }

    fn push_single_non_fungible(
        &self,
        token_id: &TokenIdentifier,
        nonce: Nonce,
        amount: &Self::BigUint,
        precision: u64,
        queues: &mut Vec<u64>,
    ) -> SCResult<()> {
        require!(nonce != 0, "Nonce cannot be zero");

        if self.reserve(token_id, nonce).is_empty() {
            self.push_single_non_fungible_new(
                token_id,
                nonce,
                amount,
                precision,
                queues,
                Option::None,
            );
        } else {
            self.push_single_non_fungible_existing(token_id, nonce, amount, precision, queues);
        }

        Ok(())
    }

    fn should_change_queue(&self, current: u64, new: u64) -> bool {
        current != new
    }

    fn get_precision_or_default(&self, token_id: &TokenIdentifier) -> u64 {
        let precision = self.precision(token_id).get();

        if precision != 0 {
            precision
        } else {
            DEFAULT_PRECISION
        }
    }

    fn get_compressed_pairs(
        &self,
        pairs: &[NonceAmountPair<Self::BigUint>],
    ) -> Vec<NonceAmountPair<Self::BigUint>> {
        let mut pairs_compressed = Vec::<NonceAmountPair<Self::BigUint>>::new();

        pairs.iter().filter(|&x| x.amount != 0).for_each(|pair| {
            match pairs_compressed.iter().position(|x| x.nonce == pair.nonce) {
                Some(index) => pairs_compressed[index].amount += &pair.amount,
                None => pairs_compressed.push(pair.clone()),
            }
        });

        pairs_compressed
    }

    fn get_queue_id(&self, amount: &Self::BigUint, precision: u64) -> u64 {
        let mut queue_id = 0;

        let precision_biguint = Self::BigUint::from(precision);
        let mut amount_clone = amount.clone();
        while amount_clone != 0 {
            amount_clone /= &precision_biguint;
            queue_id += 1;
        }

        queue_id
    }

    fn push_single_non_fungible_new(
        &self,
        token_id: &TokenIdentifier,
        nonce: Nonce,
        amount: &Self::BigUint,
        precision: u64,
        queues: &mut Vec<u64>,
        queue_id_opt: Option<u64>,
    ) {
        let queue_id = queue_id_opt.unwrap_or_else(|| self.get_queue_id(amount, precision));
        let head = self.head(token_id, queue_id).get();

        if head == 0 {
            self.init_queue(token_id, queue_id, nonce, amount);
            queues.push(queue_id);
            queues.sort();
        } else {
            self.push_to_queue(token_id, queue_id, nonce, amount);
        }
    }

    fn push_single_non_fungible_existing(
        &self,
        token_id: &TokenIdentifier,
        nonce: Nonce,
        amount: &Self::BigUint,
        precision: u64,
        queues: &mut Vec<u64>,
    ) {
        let mut elem = self.reserve(token_id, nonce).get();
        let queue = elem.queue_id;
        elem.amount += amount;
        let new_queue_id = self.get_queue_id(&elem.amount, precision);

        if new_queue_id != elem.queue_id {
            let head = self.head(token_id, queue).get();
            let tail = self.head(token_id, queue).get();
            let is_head = nonce == head;
            let is_tail = nonce == tail;

            if is_head && is_tail {
                self.head(token_id, queue).clear();
                self.tail(token_id, queue).clear();

                let index = queues.iter().position(|x| *x == queue).unwrap();
                queues.remove(index);
            } else if is_head {
                self.head(token_id, queue).set(&elem.next);
                self.reserve(token_id, elem.next).update(|x| x.prev = 0);
            } else if is_tail {
                self.tail(token_id, queue).set(&elem.prev);
                self.reserve(token_id, elem.prev).update(|x| x.next = 0);
            } else {
                self.reserve(token_id, elem.next)
                    .update(|x| x.prev = elem.prev);
                self.reserve(token_id, elem.prev)
                    .update(|x| x.next = elem.next);
            }

            self.reserve(token_id, nonce).clear();
            self.push_single_non_fungible_new(
                token_id,
                nonce,
                &elem.amount,
                precision,
                queues,
                Some(new_queue_id),
            );
        } else {
            self.reserve(token_id, nonce).update(|x| x.amount += amount);
        }
    }

    fn init_queue(
        &self,
        token_id: &TokenIdentifier,
        queue_id: u64,
        nonce: Nonce,
        amount: &Self::BigUint,
    ) {
        let elem = ReserveNode::from(amount.clone(), queue_id);
        self.reserve(token_id, nonce).set(&elem);

        self.head(token_id, queue_id).set(&nonce);
        self.tail(token_id, queue_id).set(&nonce);
    }

    fn push_to_queue(
        &self,
        token_id: &TokenIdentifier,
        queue_id: u64,
        nonce: Nonce,
        amount: &Self::BigUint,
    ) {
        let mut elem = ReserveNode::from(amount.clone(), queue_id);
        let old_tail = self.tail(token_id, queue_id).get();
        let mut old_tail_elem = self.reserve(token_id, old_tail).get();

        old_tail_elem.next = nonce;
        elem.prev = old_tail;

        self.reserve(token_id, nonce).set(&elem);
        self.reserve(token_id, old_tail).set(&old_tail_elem);

        self.tail(token_id, queue_id).set(&nonce);
    }

    #[storage_mapper("reserve")]
    fn reserve(
        &self,
        token_id: &TokenIdentifier,
        nonce: Nonce,
    ) -> SingleValueMapper<Self::Storage, ReserveNode<Self::BigUint>>;

    #[storage_mapper("head")]
    fn head(
        &self,
        token_id: &TokenIdentifier,
        queue_id: u64,
    ) -> SingleValueMapper<Self::Storage, Nonce>;

    #[storage_mapper("tail")]
    fn tail(
        &self,
        token_id: &TokenIdentifier,
        queue_id: u64,
    ) -> SingleValueMapper<Self::Storage, Nonce>;

    #[storage_mapper("queues")]
    fn queues(&self, token_id: &TokenIdentifier) -> SingleValueMapper<Self::Storage, Vec<u64>>;

    #[storage_mapper("precision")]
    fn precision(&self, token_id: &TokenIdentifier) -> SingleValueMapper<Self::Storage, u64>;
}
