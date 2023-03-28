use common_structs::LockedAssetTokenAttributes;
use common_structs::LockedAssetTokenAttributesEx;
use common_structs::UnlockMilestoneEx;
use common_structs::UnlockScheduleEx;
use common_structs::PRECISION_EX_INCREASE;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::module]
pub trait AttrExHelper {
    fn get_attributes_ex(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: u64,
    ) -> LockedAssetTokenAttributesEx<Self::Api> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        let attr_ex_activation = self.extended_attributes_activation_nonce().get();
        if token_nonce < attr_ex_activation {
            let attr: LockedAssetTokenAttributes<Self::Api> = token_info.decode_attributes();
            self.convert_attr_to_attr_ex(&attr)
        } else {
            token_info.decode_attributes()
        }
    }

    fn convert_attr_to_attr_ex(
        &self,
        attr: &LockedAssetTokenAttributes<Self::Api>,
    ) -> LockedAssetTokenAttributesEx<Self::Api> {
        let mut new_milestones: ManagedVec<UnlockMilestoneEx> = ManagedVec::new();

        for milestones in attr.unlock_schedule.unlock_milestones.iter() {
            new_milestones.push(UnlockMilestoneEx {
                unlock_epoch: milestones.unlock_epoch,
                unlock_percent: (milestones.unlock_percent as u64) * PRECISION_EX_INCREASE,
            });
        }

        LockedAssetTokenAttributesEx {
            unlock_schedule: UnlockScheduleEx {
                unlock_milestones: new_milestones,
            },
            is_merged: attr.is_merged,
        }
    }

    #[view(getExtendedAttributesActivationNonce)]
    #[storage_mapper("extended_attributes_activation_nonce")]
    fn extended_attributes_activation_nonce(&self) -> SingleValueMapper<u64>;
}
