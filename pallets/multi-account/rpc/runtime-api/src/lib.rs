#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use codec::Codec;
    // create rpc call to check number of signed sig for a call hash ❌ 
	// create rpc for getting signatories for an account ❌
	// create rpc for getting the number of threshold required for a multi account ❌
type CallHash = [u8; 32];

sp_api::decl_runtime_apis! {
	/// This trait contains all the Api's that can be called into from the runtime
	/// into our pallet. To read or perform certain state actions in our blockchain
	pub trait MultiAccountApi<AccountId>
    where AccountId: Codec {
		/// get the number of accounts that have approved a particular call hash
        fn get_approvals_for_call(id: AccountId, call: CallHash ) -> Option<u32>;
        // /// get the signatories for an account
        fn get_signatories_for_account(id: AccountId) -> Option<Vec<AccountId>>;
        /// get the threshold required for a call to pass
        fn get_threshold_for_account(id: AccountId) -> Option<u32>;
        /// get the accounts that has approved a particular call
        fn get_approval_accounts_for_call(id: AccountId, call: CallHash) -> Option<Vec<AccountId>>;
	}
}
