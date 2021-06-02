#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use frame_support::{decl_module,
					decl_storage,
					decl_event,
					decl_error,
					dispatch,
					traits::{ Get, Randomness},
					StorageValue,
					StorageDoubleMap,
					RuntimeDebug};
use frame_system::ensure_signed;
use sp_io::hashing::blake2_128;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct Kitty([u8; 16]);

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
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
	pub enum Event<T> where AccountId = <T as frame_system::Trait>::AccountId, {
			KittyCreated(AccountId, u32, Kitty),
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Trait> {
		NoneValue,
		StorageOverflow,
		KittiesIdOverflow
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		#[weight = 1000]
		pub fn create(origin) {
			let sender = ensure_signed(origin)?;

			let payload = (
				<pallet_randomness_collective_flip::Module<T> as Randomness<T::Hash>>::random_seed(),
				&sender,
				<frame_system::Module<T>>::extrinsic_index(),
			);

			let dna = payload.using_encoded(blake2_128);

			let kitty = Kitty(dna);
			let kitty_id = Self::next_kitty_id();

			Kitties::<T>::insert(&sender, kitty_id, kitty.clone());

			let new_kitty_id = kitty_id
				.checked_add(1)
				.ok_or(Error::<T>::KittiesIdOverflow)?;

			NextKittyId::put(new_kitty_id + 1);

			Self::deposit_event(RawEvent::KittyCreated(sender, kitty_id, kitty))
		}
	}
}
