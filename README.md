# xstorage

This pallet is used for other parachains to place storage order through XCM. It can support the self native token and crust native token as the fee.


# Integration
1. add the pallet as the dependency
```
xstorage-client = { git = "https://github.com/crustio/xstorage", default-features = false, rev = "087085cd6c240b4543ff3b0570c8047dfe878269"}
```

2. Config it in the runtime
```
parameter_types! {
	pub FeePerSecond: u128 = 1_000_000;
}

impl xstorage_client::Config for Runtime {
	type Event = Event;
	type XcmpMessageSender = XcmRouter;
	type AssetTransactor = AssetTransactors;
	type CurrencyId = CurrencyId;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type CurrencyIdToMultiLocation =
		CurrencyIdtoMultiLocation<AsAssetType<AssetId, AssetType, AssetManager>>;
	type LocationInverter = LocationInverter<Ancestry>;
	type CrustNativeToken = xstorage_client::primitives::CrustShadowLocation;
	type SelfNativeToken = SelfLocation;
	type FeePerSecond = FeePerSecond;
	type Destination = xstorage_client::primitives::CsmMultiloaction;
}
```

The `FeePerSecond` should be same with the settings on the Crust Chain for your native token.
