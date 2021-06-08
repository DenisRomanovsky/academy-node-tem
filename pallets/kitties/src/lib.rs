#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,  dispatch::{DispatchError, DispatchResult}, ensure,
    traits::Randomness, RuntimeDebug, StorageDoubleMap, StorageValue,
};
use frame_system::ensure_signed;
use sp_io::hashing::blake2_128;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct Kitty([u8; 16]);

#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq)]
pub enum KittyGender {
    Male,
    Female,
}

impl Kitty {
    pub fn gender(&self) -> KittyGender {
        if self.0[0] > 4 {
            KittyGender::Male
        } else {
            KittyGender::Female
        }
    }

    pub fn dna(&self) -> [u8; 16] {
        self.0
    }
}

/// This one defines types used by this exact pallet. After this, in Runtime lib.rs we may define
/// what types are given to this pallet.
/// We may use same pallet for several times using different input types. Later this is renamed to
/// Confid instead of Trait to display actual usage.
pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    type Randomness: Randomness<Self::Hash>;
}

// The pallet's runtime storage items.
// https://substrate.dev/docs/en/knowledgebase/runtime/storage
decl_storage! {
    trait Store for Module<T: Trait> as Kitties {
        pub Kitties get(fn kitties): double_map hasher(blake2_128_concat) T::AccountId, hasher(blake2_128_concat) u32 => Option<Kitty>;

        pub NextKittyId get(fn next_kitty_id): u32;
    }
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
    {
        /// Kitty created. owner / kitty id / Kitty
        KittyCreated(AccountId, u32, Kitty),
        /// Kitty breed. owner / Kitty / Kitty / Resulting kitty
        KittyBreed(AccountId, Kitty, Kitty, Kitty),
        /// Kitty transferred. old owner / new owner / kitty
        KittyTransferred(AccountId, AccountId, Kitty),
    }
);

// Errors inform users that something went wrong.
decl_error! {
    pub enum Error for Module<T: Trait> {
        NoneValue,
        StorageOverflow,
        KittiesIdOverflow,
        SameGenderBreed,
        KittenNotFound,
        WrongDNA,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = 1000]
        pub fn create(origin) {
            let sender = ensure_signed(origin)?;

            NextKittyId::try_mutate(|next_id| -> DispatchResult {
                let dna = Self::random_value(&sender);
                let kitty = Kitty(dna);
                let kitty_id = Self::get_next_kitty_id()?;

                Kitties::<T>::insert(&sender, kitty_id, kitty.clone());
                Self::deposit_event(RawEvent::KittyCreated(sender, kitty_id, kitty));

                Ok(())
            })?;
        }

        #[weight = 1000]
        pub fn breed(origin, first_kitty_id: u32, second_kitty_id: u32) {
            let sender = ensure_signed(origin)?;
            let first_kitty = Self::kitties(&sender, first_kitty_id).ok_or_else(|| Error::<T>::KittenNotFound)?;
            let second_kitty = Self::kitties(&sender, second_kitty_id).ok_or_else(|| Error::<T>::KittenNotFound)?;


            ensure!(first_kitty.gender() != second_kitty.gender(), Error::<T>::SameGenderBreed);

            let mut new_kitty_dna = [0u8; 16];
            let random_dna_selector = Self::random_value(&sender);

            for i in 0..new_kitty_dna.len() {
                new_kitty_dna[i] = combine_dna(
                    first_kitty.dna()[i],
                    second_kitty.dna()[i],
                    random_dna_selector[i]);
            }

            let kitty_id = Self::get_next_kitty_id()?;
            let new_kitty = Kitty(new_kitty_dna);
            Kitties::<T>::insert(&sender, kitty_id, &new_kitty);
            Self::deposit_event(RawEvent::KittyBreed(sender, first_kitty, second_kitty, new_kitty))
        }

        #[weight = 1000]
        pub fn transfer(origin, kitty_id: u32, new_owner_id: T::AccountId) {
            let sender = ensure_signed(origin)?;


            Kitties::<T>::try_mutate_exists(sender.clone(), kitty_id, |kitty| -> DispatchResult {
                if sender == new_owner_id {
                    ensure!(kitty.is_some(), Error::<T>::KittenNotFound);
                    return Ok(());
                }

                let kitty = kitty.take().ok_or(Error::<T>::KittenNotFound)?;

                Kitties::<T>::insert(&new_owner_id, kitty_id, &kitty);

                Self::deposit_event(RawEvent::KittyTransferred(sender, new_owner_id, kitty));

                Ok(())
            })?;
        }
    }
}

fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
    (!selector & dna1) | (selector & dna2)
}

impl<T: Trait> Module<T> {
	fn get_next_kitty_id() -> sp_std::result::Result<u32, DispatchError> {
		NextKittyId::try_mutate(|next_id| -> sp_std::result::Result<u32, DispatchError> {
			let current_id = *next_id;
			*next_id = next_id.checked_add(1).ok_or(Error::<T>::KittiesIdOverflow)?;
			Ok(current_id)
		})
	}

    fn random_value(sender: &T::AccountId) -> [u8; 16] {
        let payload = (
            T::Randomness::random_seed(),
            &sender,
            <frame_system::Module<T>>::extrinsic_index(),
        );
        payload.using_encoded(blake2_128)
    }
}
