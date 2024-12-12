#![cfg_attr(not(feature = "std"), no_std)]

pub use multi_runtime_api::MultiAccountApi as AccountApi;
use jsonrpsee::{
	core::{Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;
use codec::Codec;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Custom {
	code: u32,
	sum: u32,
}
type CallHash = [u8; 32];

#[rpc(client, server)]
pub trait MultiAccountApi<BlockHash, AccountId> {

    /// get the number of accounts that have approved a particular call hash
    #[method(name = "multi_NumberOfAccountsHasApprovedCall")]
    fn get_approvals_for_call(&self, id: AccountId, call_hash: CallHash, at: Option<BlockHash> ) -> RpcResult<Option<u32>>;
    
    /// get the signatories for an account
    #[method(name = "multi_AccountSigners")]    
    fn get_signatories_for_account(&self, id: AccountId, at: Option<BlockHash> ) -> RpcResult<Option<Vec<AccountId>>>;
    
    /// get the threshold required for a call to pass
    #[method(name = "multi_AccountThreshold")]
    fn get_threshold_for_account(&self, id: AccountId, at: Option<BlockHash> ) -> RpcResult<Option<u32>>;
    
    /// get the accounts that has approved a particular call
    #[method(name = "multi_SignersWhoApprovedCall")]
    fn get_approval_accounts_for_call(&self, id: AccountId, call_hash:CallHash, at: Option<BlockHash> ) -> RpcResult<Option<Vec<AccountId>>>;
}

/// A struct that implements the `TemplateApi`.
pub struct MultiAccountPallet<C, Block> {
	// If you have more generics, no need to TemplatePallet<C, M, N, P, ...>
	// just use a tuple like TemplatePallet<C, (M, N, P, ...)>
	client: Arc<C>,
	_marker: std::marker::PhantomData<Block>,
}

impl<C, Block> MultiAccountPallet<C, Block> {
	/// Create new `TemplatePallet` instance with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

impl<C, Block, AccountId: Codec> MultiAccountApiServer<<Block as BlockT>::Hash, AccountId> for MultiAccountPallet<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: AccountApi<Block, AccountId>,
{
	fn get_approvals_for_call(&self, id: AccountId, call: CallHash, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Option<u32>> {
		let api = self.client.runtime_api();
		let block_hash = at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash);
		api.get_approvals_for_call(block_hash, id, call).map_err(runtime_error_into_rpc_err)
	}

    fn get_signatories_for_account(&self, id: AccountId,at: Option<<Block as BlockT>::Hash>) -> RpcResult<Option<Vec<AccountId>>> {
		let api = self.client.runtime_api();
		let block_hash = at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash);
		api.get_signatories_for_account(block_hash, id).map_err(runtime_error_into_rpc_err)
	}

    fn get_threshold_for_account(&self, id: AccountId,at: Option<<Block as BlockT>::Hash>) -> RpcResult<Option<u32>> {
		let api = self.client.runtime_api();
		let block_hash = at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash);
		api.get_threshold_for_account(block_hash, id).map_err(runtime_error_into_rpc_err)
	}

    fn get_approval_accounts_for_call(&self, id: AccountId, call: CallHash, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Option<Vec<AccountId>>> {
		let api = self.client.runtime_api();
		let block_hash = at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash);
		api.get_approval_accounts_for_call(block_hash, id, call).map_err(runtime_error_into_rpc_err)
	}

}

const RUNTIME_ERROR: i32 = 1;

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
	CallError::Custom(ErrorObject::owned(
		RUNTIME_ERROR,
		"Runtime error",
		Some(format!("{:?}", err)),
	))
	.into()
}
