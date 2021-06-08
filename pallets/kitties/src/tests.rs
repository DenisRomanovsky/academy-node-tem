use super::*;

use frame_support::{
    assert_noop, assert_ok, impl_outer_event, impl_outer_origin, parameter_types, weights::Weight,
};
use sp_core::H256;
use std::cell::RefCell;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
}

mod kitties {
    // Re-export needed for `impl_outer_event!`.
    pub use super::super::*;
}

impl_outer_event! {
    pub enum Event for Test {
        frame_system<T>,
        kitties<T>,
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Trait for Test {
    type BaseCallFilter = ();
    type Origin = Origin;
    type Call = ();
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type DbWeight = ();
    type BlockExecutionWeight = ();
    type ExtrinsicBaseWeight = ();
    type MaximumExtrinsicWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type PalletInfo = ();
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}

impl Trait for Test {
    type Event = Event;
    type Randomness = pallet_randomness_collective_flip::Module<Test>;
}

type KittiesModule = Module<Test>;
type System = frame_system::Module<Test>;

thread_local! {
    static RANDOM_PAYLOAD: RefCell<H256> = RefCell::new(Default::default());
}

pub struct MockRandom;

impl Randomness<H256> for MockRandom {
	fn random(_subject: &[u8]) -> H256 {
		RANDOM_PAYLOAD.with(|v| *v.borrow())
	}
}

fn set_random(val: H256) {
	RANDOM_PAYLOAD.with(|v| *v.borrow_mut() = val)
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap()
        .into();
    t.execute_with(|| System::set_block_number(1));
    t
}

fn last_event() -> Event {
    System::events().last().unwrap().event.clone()
}

#[test]
fn can_create() {
    new_test_ext().execute_with(|| {
        assert_ok!(KittiesModule::create(Origin::signed(100)));

        let kitty = Kitty([
            59, 250, 138, 82, 209, 39, 141, 109, 163, 238, 183, 145, 235, 168, 18, 122,
        ]);

        assert_eq!(KittiesModule::kitties(100, 0), Some(kitty.clone()));
        assert_eq!(KittiesModule::next_kitty_id(), 1);

        assert_eq!(
            last_event(),
            Event::kitties(RawEvent::KittyCreated(100, 0, kitty))
        );
    });
}

#[test]
fn gender() {
	assert_eq!(Kitty([0; 16]).gender(), KittyGender::Male);
	assert_eq!(Kitty([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).gender(), KittyGender::Female);
}

#[test]
fn can_breed() {
    new_test_ext().execute_with(|| {
        assert_ok!(KittiesModule::create(Origin::signed(100)));
		set_random(H256::from([2; 32]));

		let kitty_one = KittiesModule::kitties(100, 0).unwrap();

        System::set_extrinsic_index(1);

        assert_ok!(KittiesModule::create(Origin::signed(100)));
		let kitty_two = KittiesModule::kitties(100, 1).unwrap();

        assert_noop!(
            KittiesModule::breed(Origin::signed(100), 0, 11),
            Error::<Test>::KittenNotFound
        );
        assert_noop!(
            KittiesModule::breed(Origin::signed(100), 0, 0),
            Error::<Test>::SameGenderBreed
        );
        assert_noop!(
            KittiesModule::breed(Origin::signed(101), 0, 1),
            Error::<Test>::KittenNotFound
        );

        assert_ok!(KittiesModule::breed(Origin::signed(100), 0, 1));

        let kitty = Kitty([
			59, 254, 219, 122, 245, 239, 191, 125, 255, 239, 247, 247, 251, 239, 247, 254
        ]);

        assert_eq!(KittiesModule::kitties(100, 2), Some(kitty.clone()));
        assert_eq!(KittiesModule::next_kitty_id(), 3);

        assert_eq!(
            last_event(),
            Event::kitties(RawEvent::KittyBreed(100, kitty_one, kitty_two, kitty))
        );
    });
}

#[test]
fn combine_dna_works() {
    assert_eq!(combine_dna(0b11111111, 0b00000000, 0b00001111), 0b11110000);
    assert_eq!(combine_dna(0b10101010, 0b11110000, 0b11001100), 0b11100010);
}
