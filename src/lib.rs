// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet;
pub use pallet::*;

pub mod primitives;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

#[pallet]
pub mod pallet {
	use sp_std::prelude::*;
	use frame_support::{pallet_prelude::*, weights::constants::WEIGHT_PER_SECOND};
	use frame_system::pallet_prelude::*;

	use xcm::v2::prelude::*;
	use codec::Encode;
	use sp_std::convert::TryInto;
	use sp_runtime::traits::Convert;

	use xcm_executor::traits::{InvertLocation, TransactAsset};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type XcmpMessageSender: SendXcm;

		/// AssetTransactor allows us to transfer asset
		type AssetTransactor: TransactAsset;

		/// Currency Id.
		type CurrencyId: Parameter + Member + Clone;

		/// Convert `T::CurrencyId` to `MultiLocation`.
		type CurrencyIdToMultiLocation: Convert<Self::CurrencyId, Option<MultiLocation>>;

		/// Convert `T::AccountId` to `MultiLocation`.
		type AccountIdToMultiLocation: Convert<Self::AccountId, MultiLocation>;

		/// Means of inverting a location.
		type LocationInverter: InvertLocation;

		type CrustNativeToken: Get<MultiLocation>;

		type SelfNativeToken: Get<MultiLocation>;

		type FeePerSecond: Get<u128>;

		type Destination: Get<MultiLocation>;
	}

	/// An error that can occur while executing the mapping pallet's logic.
	#[pallet::error]
	pub enum Error<T> {
		NotCrossChainTransferableCurrency,
		NotSupportedCurrency,
		UnableToTransferStorageFee,
		WeightOverflow,
		ErrorSending,
		CannotReanchor,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New asset with the asset manager is registered
		FileSuccess {
			account: T::AccountId,
			cid: Vec<u8>,
			size: u64
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn storage_fee_per_currency)]
	pub type StorageFeePerCurrency<T: Config> =
		StorageMap<_, Blake2_128Concat, T::CurrencyId, u128>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000_000)]
		pub fn place_storage_order(
			origin: OriginFor<T>,
			currency_id: T::CurrencyId,
			cid: Vec<u8>,
			size: u64,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let fee_type: MultiLocation =
				T::CurrencyIdToMultiLocation::convert(currency_id.clone()).ok_or(Error::<T>::NotCrossChainTransferableCurrency)?;

			ensure!(fee_type == T::CrustNativeToken::get() || fee_type == T::SelfNativeToken::get(), Error::<T>::NotSupportedCurrency);

			let origin_as_mult = T::AccountIdToMultiLocation::convert(who.clone());

			let dest_weight = crate::primitives::XSTORAGE_CALL_WEIGHT;
			let total_weight = dest_weight + crate::primitives::UNIT_XCM_WEIGHT * 3;

			let order_call = (crate::primitives::XSTORAGE_PALLET_INDEX, crate::primitives::XSTORAGE_CALL_INDEX, cid.clone(), size).encode();

			let dest = T::Destination::get();

			let transact_message = if fee_type == T::CrustNativeToken::get() {

				let amount: u128 = Self::calculate_fee_in_crust_native_token(total_weight);

				// Construct MultiAsset
				let fee = MultiAsset {
					id: Concrete(fee_type),
					fun: Fungible(amount),
				};

				// Construct the local withdraw message with the previous calculated amount
				// This message deducts and burns "amount" from the caller when executed
				T::AssetTransactor::withdraw_asset(&fee.clone().into(), &origin_as_mult)
					.map_err(|_| Error::<T>::UnableToTransferStorageFee)?;

				Xcm(vec![
					Self::sovereign_withdraw(fee.clone(), &dest)?,
					Self::buy_execution(fee, &dest, total_weight)?,
					Self::order_message(order_call, dest_weight)?,
				])
			} else {

				let amount =
				Self::calculate_fee_per_second(total_weight, T::FeePerSecond::get());

				// Construct MultiAsset
				let fee = MultiAsset {
					id: Concrete(fee_type),
					fun: Fungible(amount),
				};

				T::AssetTransactor::internal_transfer_asset(&fee.clone().into(), &origin_as_mult, &dest)
				.map_err(|_| Error::<T>::UnableToTransferStorageFee)?;

				Xcm(vec![
					Self::sovereign_mint(fee.clone(), &dest)?,
					Self::buy_execution(fee, &dest, total_weight)?,
					Self::order_message(order_call, dest_weight)?,
				])
			};

			// Send to sovereign
			T::XcmpMessageSender::send_xcm(dest, transact_message).map_err(|_| Error::<T>::ErrorSending)?;

			Self::deposit_event(Event::FileSuccess {
				account: who,
				cid,
				size,
			});

			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Construct a buy execution xcm order with the provided parameters
		fn buy_execution(
			asset: MultiAsset,
			at: &MultiLocation,
			weight: u64,
		) -> Result<Instruction<()>, DispatchError> {
			let ancestry = T::LocationInverter::ancestry();
			let fees = asset
				.reanchored(at, &ancestry)
				.map_err(|_| Error::<T>::CannotReanchor)?;

			Ok(BuyExecution {
				fees,
				weight_limit: WeightLimit::Limited(weight),
			})
		}

		/// Construct a withdraw instruction for the sovereign account
		fn sovereign_withdraw(
			asset: MultiAsset,
			at: &MultiLocation,
		) -> Result<Instruction<()>, DispatchError> {
			let ancestry = T::LocationInverter::ancestry();
			let fees = asset
				.reanchored(at, &ancestry)
				.map_err(|_| Error::<T>::CannotReanchor)?;

			Ok(WithdrawAsset(fees.into()))
		}

		/// Construct a reserve assest deposited instruction for the sovereign account
		fn sovereign_mint(
			asset: MultiAsset,
			at: &MultiLocation,
		) -> Result<Instruction<()>, DispatchError> {
			let ancestry = T::LocationInverter::ancestry();
			let fees = asset
				.reanchored(at, &ancestry)
				.map_err(|_| Error::<T>::CannotReanchor)?;

			Ok(ReserveAssetDeposited(fees.into()))
		}

		/// Construct the transact xcm message with the provided parameters
		fn order_message(
			call: Vec<u8>,
			dispatch_weight: Weight,
		) -> Result<Instruction<()>, DispatchError> {
			Ok(Transact {
					origin_type: OriginKind::SovereignAccount,
					require_weight_at_most: dispatch_weight,
					call: call.into(),
				},
			)
		}

		/// Returns the fee for a given set of parameters
		pub fn calculate_fee_per_second(weight: Weight, fee_per_second: u128) -> u128 {
			let weight_fee =
				fee_per_second.saturating_mul(weight as u128) / (WEIGHT_PER_SECOND as u128);
			return weight_fee;
		}


		pub fn calculate_fee_in_crust_native_token(weight: Weight) -> u128 {
			return 10u128.saturating_mul(weight as u128);
		}
		
	}
}