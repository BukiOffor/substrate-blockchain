// All pallets must be configured for `no_std`.
#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

//mod weights;

#[frame_support::pallet]
pub mod pallet {

	use super::*;

	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	use frame_support::inherent::Vec;
	use scale_info::prelude::{boxed::Box, vec};

	use frame_system::RawOrigin;

	use frame_support::{
		dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
		Parameter,
	};
	use sp_runtime::traits::TrailingZeroInput;

	// create an account for a set of signatories in this pallet
	// set account nonce to 0
	// store the account on our storage map with the account Id of of all its signatories and a
	// required signature threshold. allow the account to hold balances.
	// dispatch any call if the signature threshold is met.

	// create rpc call to check number of signed sig for a call hash ❌ 
	// create rpc for getting signatories for an account ❌
	// create rpc for getting the number of threshold required for a multi account ❌

	type CallHash = [u8; 32];

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// A type representing the weights required by the dispatchables of this pallet.
		type WeightInfo;

		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The overarching call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ From<frame_system::Call<Self>>;
		type MaxSignatories: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn get_account)]
	pub type Account<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<T::AccountId, T::MaxSignatories>,
		ValueQuery,
	>;

	/// This is a terrible use case storing this data seperately on the blockchain.
	/// because this will require making multiple calls to fetch information that can be
	/// fetched with a single call. The `Account` storage should be set to a `NStorageMap`
	/// that will be able to store all these information.
	#[pallet::storage]
	#[pallet::getter(fn get_threshold)]
	pub type Threshold<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u16, ValueQuery>;

	/// This storage location is a double map of MultiAccount Id -> Hash(Call) -> array of
	/// signatories that have voted yes The bounded vec is important because it keeps track of
	/// accounts that have voted yes on a transaction with the bounded vec we can be sure that
	/// there is no double voting.
	#[pallet::storage]
	pub type Calls<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Blake2_128Concat,
		CallHash,
		BoundedVec<T::AccountId, ConstU32<100>>,
		ValueQuery,
	>;

	/// This is a storage item for executed calls
	#[pallet::storage]
	pub type Executed<T: Config> = StorageMap<_, Blake2_128Concat, CallHash, (), ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Account { id: T::AccountId, signatories: Vec<T::AccountId>, threshold: u16 },
		Approval { id: T::AccountId, signer: T::AccountId, hash: CallHash },
		Call { id: T::AccountId, hash: CallHash, signatories: Vec<T::AccountId> },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(10))]
		pub fn register_account(
			origin: OriginFor<T>,
			id: T::AccountId,
			other_signatories: Vec<T::AccountId>,
			threshold: u16,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(threshold >= 1, Error::<T>::MinimumThreshold);
			let signatories = Self::ensure_sorted_and_insert(other_signatories, who.clone())?;
			let bounded_vec =
				BoundedVec::try_from(signatories).map_err(|_| Error::<T>::TooManySignatories)?;
			<Account<T>>::insert(&id, &bounded_vec);
			<Threshold<T>>::insert(&id, &threshold);
			Self::deposit_event(Event::Account {
				id,
				signatories: bounded_vec.into_inner(),
				threshold,
			});
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(5))]
		pub fn account_create_call(
			origin: OriginFor<T>,
			id: T::AccountId,
			call: Box<<T as Config>::RuntimeCall>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let signatories = <Account<T>>::get(&id);
			ensure!(
				signatories.into_inner().binary_search(&who).is_ok(),
				Error::<T>::SignerIsNotApproved
			);
			let hash = call.using_encoded(sp_io::hashing::blake2_256);

			//check if hash already exists
			let calls = <Calls<T>>::get(&id, &hash);
			ensure!(calls.is_empty(), Error::<T>::CallHashAlreadyExists);

			// whoever creates the call has already approved it because we are inserting the caller
			// into the vec
			let approvals = BoundedVec::try_from(vec![who.clone()])
				.map_err(|_| Error::<T>::TooManySignatories)?;
			<Calls<T>>::insert(&id, &hash, approvals);
			Self::deposit_event(Event::Approval { id, signer: who, hash });
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(5))]
		pub fn approve_or_dispatch_call(
			origin: OriginFor<T>,
			id: T::AccountId,
			call: Box<<T as Config>::RuntimeCall>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let signatories = <Account<T>>::get(&id);
			ensure!(
				signatories.into_inner().binary_search(&who).is_ok(),
				Error::<T>::SignerIsNotApproved
			);
			let approvals_needed = <Threshold<T>>::get(&id);
			let mut signers: Vec<_> = Vec::new();
			let mut number_of_approvals = 0;
			let hash = &call.using_encoded(sp_io::hashing::blake2_256);
			// ensure!(
			//     <Calls<T>>::contains_key(&id, hash),
			//     Error::<T>::NotFound
			// );
			<Calls<T>>::try_mutate(&id, hash, |sig| -> DispatchResultWithPostInfo {
				// if the number of approvals needed has passed and the call has been dispatched
				// there is no need to add 32 bytes in storage that is of no use
				// so we return early here
				if sig.as_slice().len() as u16 == approvals_needed {
					return Err(Error::<T>::DispatchHasAlreadyOccured.into());
				};
				// the ensure_sorted_and_insert already makes a check to confirm if an account id
				// already exists in the bounded vec. so we can be sure that a double vote will not
				// occur.
				number_of_approvals = sig.as_slice().len() as u16;
				let sorted_vec =
					Self::ensure_sorted_and_insert(sig.as_slice().to_vec(), who.clone())?;
				signers.extend(sorted_vec.clone());
				*sig =
					BoundedVec::try_from(sorted_vec).map_err(|_| Error::<T>::TooManySignatories)?;
				Ok(().into())
			})?;

			Self::deposit_event(Event::Approval {
				id: id.clone(),
				signer: who.clone(),
				hash: hash.clone(),
			});

			if (number_of_approvals + 1) == approvals_needed {
				//call.dispatch(<T as frame_system::Config>::RuntimeOrigin::signed(id));
				let result = call.dispatch(RawOrigin::Signed(id.clone()).into());
				result.map_err(|err| {
					return err;
				})?;
				Self::deposit_event(Event::Call {
					id: id.clone(),
					signatories: signers,
					hash: hash.clone(),
				});
				// remove call from storage because of space ??
				// <Calls<T>>::remove(id.clone(), hash);
			}

			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Derive a multi-account ID from the sorted list of accounts and the threshold that are
		/// required.
		///
		/// NOTE: `who` must be sorted. If it is not, then you'll get the wrong answer.
		pub fn multi_account_id(who: &[T::AccountId], threshold: u16) -> T::AccountId {
			let entropy =
				(b"modlpy/utilisuba", who, threshold).using_encoded(sp_io::hashing::blake2_256);
			Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
				.expect("infinite length input; no invalid inputs for type; qed")
		}

		/// Check that signatories is sorted and doesn't contain sender, then insert sender.
		fn ensure_sorted_and_insert(
			other_signatories: Vec<T::AccountId>,
			who: T::AccountId,
		) -> Result<Vec<T::AccountId>, DispatchError> {
			let mut signatories = other_signatories;
			let mut maybe_last = None;
			let mut index = 0;
			for item in signatories.iter() {
				if let Some(last) = maybe_last {
					ensure!(last < item, Error::<T>::SignatoriesOutOfOrder);
				}
				if item <= &who {
					ensure!(item != &who, Error::<T>::SenderInSignatories);
					index += 1;
				}
				maybe_last = Some(item);
			}
			signatories.insert(index, who);
			Ok(signatories)
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Threshold must be 2 or greater.
		MinimumThreshold,
		/// Call doesn't need any (more) approvals.
		NoApprovalsNeeded,
		/// There are too few signatories in the list.
		TooFewSignatories,
		/// There are too many signatories in the list.
		TooManySignatories,
		/// The signatories were provided out of order; they should be ordered.
		SignatoriesOutOfOrder,
		/// The sender was contained in the other signatories; it shouldn't be.
		SenderInSignatories,
		/// Multisig operation not found when attempting to cancel.
		NotFound,
		/// Signer is not part of the approved signatories
		SignerIsNotApproved,
		/// Dispatch has already happend
		DispatchHasAlreadyOccured,
		/// Call hash has already been created by a user
		CallHashAlreadyExists,
	}
}
