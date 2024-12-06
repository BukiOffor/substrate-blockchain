#![cfg_attr(not(feature = "std"), no_std)]

// declear our runtime api's. Using the declear runtime API macro.
sp_api::decl_runtime_apis! {
	/// This trait contains all the Api's that can be called into from the runtime
	/// into our pallet. To read or perform certain state actions in our blockchain
	pub trait ConnectApi {
		fn total_registered() -> u32;
	}
}
