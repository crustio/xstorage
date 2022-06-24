// Crust Parachain on Kusama's
use xcm::latest::{
	prelude::*, MultiLocation,
};

pub use frame_support::parameter_types;

parameter_types! {
    pub const CrustShadowLocation: MultiLocation = MultiLocation {
        parents: 1,
        interior: X1(Parachain(2012)),
    };
    pub const CsmMultiloaction: MultiLocation = MultiLocation {
        parents: 1,
        interior: X1(Parachain(2012)),
    };
}

pub const UNIT_XCM_WEIGHT: u64 = 1_000_000_000;
pub const XSTORAGE_PALLET_INDEX: u8 = 127;
pub const XSTORAGE_CALL_INDEX: u8 = 0;
pub const XSTORAGE_CALL_WEIGHT: u64 = 1_000_001;