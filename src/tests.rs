use crate::mock::*;
use crate::*;
use frame_support::{
	assert_noop, assert_ok, storage::migration::put_storage_value,
};

#[test]
fn test_place_storage_order_success() {
	ExtBuilder::default()
		.with_balances(vec![(1, 1000000000000000)])
		.build()
		.execute_with(|| {
			// It should work by using self reserved token
			assert_ok!(Xstorage::place_storage_order(
				Origin::signed(1u64),
				CurrencyId::SelfReserve,
				vec![1,2,3],
				100u64,
			));
			let expected = vec![
				crate::Event::FileSuccess {
					account: 1u64,
					cid: vec![1,2,3],
					size: 100u64,
				},
			];
			assert_eq!(events(), expected);

			// It should work by using crust native token
			assert_ok!(Xstorage::place_storage_order(
				Origin::signed(1u64),
				CurrencyId::OtherReserve(2),
				vec![4,5,6],
				200u64,
			));
			let expected = vec![
				crate::Event::FileSuccess {
					account: 1u64,
					cid: vec![1,2,3],
					size: 100u64,
				},
				crate::Event::FileSuccess {
					account: 1u64,
					cid: vec![4,5,6],
					size: 200u64,
				},
			];
			assert_eq!(events(), expected);
		})
}

#[test]
fn test_place_storage_order_failed() {
	ExtBuilder::default()
		.with_balances(vec![(1, 1000000000000000)])
		.build()
		.execute_with(|| {
			// other assets except for self navite token and crust native token should fail
			assert_noop!(Xstorage::place_storage_order(
				Origin::signed(1u64),
				CurrencyId::OtherReserve(10),
				vec![4,5,6],
				200u64,
			),
			Error::<Test>::NotSupportedCurrency);
		})
}
