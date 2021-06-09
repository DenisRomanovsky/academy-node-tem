#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	dispatch::DispatchResult,
	ensure,
	traits::{Currency, Randomness, ExistenceRequirement},
	RuntimeDebug, StorageDoubleMap,
};
use frame_system::ensure_signed;
use sp_io::hashing::blake2_128;

use sp_std::vec::Vec;
use orml_utilities::with_transaction_result;
use orml_nft::Module as NftModule;

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
		if self.0[0] % 2 == 0 {
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
pub trait Trait: orml_nft::Trait<TokenData = Kitty, ClassData=()> {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Randomness: Randomness<Self::Hash>;
	type Currency: Currency<Self::AccountId>;
}

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
type KittyIndexOf<T> = <T as orml_nft::Trait>::TokenId;

// The pallet's runtime storage items.
// https://substrate.dev/docs/en/knowledgebase/runtime/storage
decl_storage! {
    trait Store for Module<T: Trait> as Kitties {
        pub KittyPrices get(fn kitty_prices): map hasher(blake2_128_concat) KittyIndexOf<T> => Option<BalanceOf<T>>;

		pub ClassId get(fn class_id): T::ClassId;
    }
	add_extra_genesis {
			build(|_config| {
				// create an NTF class
				let class_id = NftModule::<T>::create_class(&Default::default(), Vec::new(), ()).expect("Cannot fail or invalid chain spec");
				ClassId::<T>::put(class_id);
			})
	}
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
		KittyIndex = KittyIndexOf<T>,
        Balance = BalanceOf<T>,
    {
        /// Kitty created. owner / kitty id / Kitty
        KittyCreated(AccountId, KittyIndex, Kitty),
        /// Kitty breed. owner / Kitty / Kitty / Resulting kitty
        KittyBreed(AccountId, Kitty, Kitty, KittyIndex),
        /// Kitty transferred. old owner / new owner / kitty
        KittyTransferred(AccountId, AccountId, KittyIndex),
        /// Kitty price set. owner / kitty id / price
        KittyPriceUpdated(AccountId, KittyIndex, Option<Balance>),
        /// Kitty sold set. seller/ byer / kitty id / price
        KittySold(AccountId, AccountId, KittyIndex, Balance),
    }
);

// Errors inform users that something went wrong.
decl_error! {
    pub enum Error for Module<T: Trait> {
        NoneValue,
        StorageOverflow,
        SameGenderBreed,
        KittenNotFound,
        WrongDNA,
        NotForSale,
        PriceTooLow,
        BuyFromSelf,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = 1000]
        pub fn create(origin) {
            let sender = ensure_signed(origin)?;
            let dna = Self::random_value(&sender);
            let kitty = Kitty(dna);
			let kitty_id = NftModule::<T>::mint(&sender, Self::class_id(), Vec::new(), kitty.clone())?;

            Self::deposit_event(RawEvent::KittyCreated(sender, kitty_id, kitty));
        }

        #[weight = 1000]
        pub fn breed(origin, first_kitty_id: KittyIndexOf<T>, second_kitty_id: KittyIndexOf<T>) {
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


            let new_kitty = Kitty(new_kitty_dna);
            let kitty_id = NftModule::<T>::mint(&sender, Self::class_id(), Vec::new(), new_kitty.clone())?;

            Self::deposit_event(RawEvent::KittyBreed(sender, first_kitty, second_kitty, kitty_id))
        }

        #[weight = 1000]
        pub fn transfer(origin, kitty_id: KittyIndexOf<T>, new_owner_id: T::AccountId) {
            let sender = ensure_signed(origin)?;
			NftModule::<T>::transfer(&sender, &new_owner_id, (Self::class_id(), kitty_id))?;

			if sender != new_owner_id {
				KittyPrices::<T>::remove(kitty_id);
				Self::deposit_event(RawEvent::KittyTransferred(sender, new_owner_id, kitty_id));
			}
        }

         #[weight = 1000]
        pub fn set_price(origin, kitty_id: KittyIndexOf<T>, new_price: Option<BalanceOf<T>>) {
             let sender = ensure_signed(origin)?;

			ensure!(orml_nft::TokensByOwner::<T>::contains_key(&sender, (Self::class_id(), kitty_id)), Error::<T>::KittenNotFound);

            KittyPrices::<T>::mutate_exists(kitty_id, |price| *price = new_price);

            Self::deposit_event(RawEvent::KittyPriceUpdated(sender, kitty_id, new_price));
        }

        #[weight = 1000]
        pub fn buy(origin, owner: T::AccountId, kitty_id: KittyIndexOf<T>, max_price: BalanceOf<T>) {
             let sender = ensure_signed(origin)?;

            ensure!(sender != owner, Error::<T>::BuyFromSelf);

            KittyPrices::<T>::try_mutate_exists(kitty_id, |price| -> DispatchResult {
				let price = price.take().ok_or(Error::<T>::NotForSale)?;

				ensure!(max_price >= price, Error::<T>::PriceTooLow);

				with_transaction_result(|| {
					NftModule::<T>::transfer(&owner, &sender, (Self::class_id(), kitty_id))?;
					T::Currency::transfer(&sender, &owner, price, ExistenceRequirement::KeepAlive)?;

					Self::deposit_event(RawEvent::KittySold(owner, sender, kitty_id, price));

					Ok(())
				})
			})?;
        }
    }
}

fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
	(!selector & dna1) | (selector & dna2)
}

impl<T: Trait> Module<T> {
	fn kitties(owner: &T::AccountId, kitty_id: KittyIndexOf<T>) -> Option<Kitty> {
		NftModule::<T>::tokens(Self::class_id(), kitty_id).and_then(|x| {
			if x.owner == *owner {
				Some(x.data)
			} else {
				None
			}
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
