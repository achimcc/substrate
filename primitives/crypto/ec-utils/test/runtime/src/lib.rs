// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The Substrate runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub use substrate_test_runtime::extrinsic;
#[cfg(feature = "std")]
pub use substrate_test_runtime::genesismap;
use substrate_test_runtime::SubstrateTest;

use codec::{Decode, Encode};
use frame_support::{
	construct_runtime,
	dispatch::DispatchClass,
	parameter_types,
	traits::{ConstU32, ConstU64},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, WEIGHT_REF_TIME_PER_SECOND},
		Weight,
	},
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	CheckNonce, CheckWeight,
};
use scale_info::TypeInfo;
use sp_std::prelude::*;

use sp_application_crypto::{ecdsa, ed25519, sr25519};
use sp_core::{OpaqueMetadata, RuntimeDebug};

use sp_api::{decl_runtime_apis, impl_runtime_apis};
pub use sp_core::hash::H256;
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
	create_runtime_str, impl_opaque_keys,
	traits::{BlakeTwo256, Block as BlockT, NumberFor, Verify},
	transaction_validity::{TransactionValidityError},
	ApplyExtrinsicResult, Perbill,
};
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub type AuraId = sp_consensus_aura::sr25519::AuthorityId;
#[cfg(feature = "std")]
pub use extrinsic::{ExtrinsicBuilder, Transfer};

const LOG_TARGET: &str = "sp-crypto-ec-utils-test-runtime";

// Include the WASM binary
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

#[cfg(feature = "std")]
pub mod wasm_binary_logging_disabled {
	include!(concat!(env!("OUT_DIR"), "/wasm_binary_logging_disabled.rs"));
}

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect(
		"Development wasm binary is not available. Testing is only supported with the flag \
		 disabled.",
	)
}

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_logging_disabled_unwrap() -> &'static [u8] {
	wasm_binary_logging_disabled::WASM_BINARY.expect(
		"Development wasm binary is not available. Testing is only supported with the flag \
		 disabled.",
	)
}

/// Test runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("test"),
	impl_name: create_runtime_str!("parity-test-ecc-host-functions"),
	authoring_version: 1,
	spec_version: 2,
	impl_version: 2,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

fn version() -> RuntimeVersion {
	VERSION
}

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// The address format for describing accounts.
pub type Address = sp_core::sr25519::Public;
pub type Signature = sr25519::Signature;
#[cfg(feature = "std")]
pub type Pair = sp_core::sr25519::Pair;

/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (CheckNonce<Runtime>, CheckWeight<Runtime>, CheckSubstrateCall);
/// The payload being signed in transactions.
pub type SignedPayload = sp_runtime::generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Unchecked extrinsic type as expected by this runtime.
pub type Extrinsic =
	sp_runtime::generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

/// An identifier for an account on this system.
pub type AccountId = <Signature as Verify>::Signer;
/// A simple hash type for all our hashing.
pub type Hash = H256;
/// The hashing algorithm used.
pub type Hashing = BlakeTwo256;
/// The block number type used in this runtime.
pub type BlockNumber = u64;
/// Index of a transaction.
pub type Index = u64;
/// The item of a block digest.
pub type DigestItem = sp_runtime::generic::DigestItem;
/// The digest of a block.
pub type Digest = sp_runtime::generic::Digest;
/// A test block.
pub type Block = sp_runtime::generic::Block<Header, Extrinsic>;
/// A test block's header.
pub type Header = sp_runtime::generic::Header<BlockNumber, Hashing>;
/// Balance of an account.
pub type Balance = u64;

decl_runtime_apis! {
	#[api_version(2)]
	pub trait TestAPI {
		/// Tests a projective mul for g1 on bls12_381
		fn test_bls12_381_g1_mul_projective_crypto(base: Vec<u8>, scalar: Vec<u8>) -> Vec<u8>;
	}
}

pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct CheckSubstrateCall;

impl sp_runtime::traits::Printable for CheckSubstrateCall {
	fn print(&self) {
		"CheckSubstrateCall".print()
	}
}

impl sp_runtime::traits::Dispatchable for CheckSubstrateCall {
	type RuntimeOrigin = CheckSubstrateCall;
	type Config = CheckSubstrateCall;
	type Info = CheckSubstrateCall;
	type PostInfo = CheckSubstrateCall;

	fn dispatch(
		self,
		_origin: Self::RuntimeOrigin,
	) -> sp_runtime::DispatchResultWithInfo<Self::PostInfo> {
		panic!("This implementation should not be used for actual dispatch.");
	}
}

impl sp_runtime::traits::SignedExtension for CheckSubstrateCall {
	type AccountId = AccountId;
	type Call = RuntimeCall;
	type AdditionalSigned = ();
	type Pre = ();
	const IDENTIFIER: &'static str = "CheckSubstrateCall";

	fn additional_signed(
		&self,
	) -> sp_std::result::Result<Self::AdditionalSigned, TransactionValidityError> {
		Ok(())
	}

	fn pre_dispatch(
		self,
		who: &Self::AccountId,
		call: &Self::Call,
		info: &sp_runtime::traits::DispatchInfoOf<Self::Call>,
		len: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		self.validate(who, call, info, len).map(drop)
	}
}

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = Extrinsic
	{
		System: frame_system,
	}
);

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// Max weight, actual value does not matter for test runtime.
const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;

	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);

	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
}

impl frame_system::pallet::Config for Runtime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = Index;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = Hashing;
	type AccountId = AccountId;
	type Lookup = sp_runtime::traits::IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<2400>;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	// For weight estimation, we assume that the most locks on an individual account will be 50.
	// This number may need to be adjusted in the future if this assumption no longer holds true.
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

parameter_types! {
	pub const EpochDuration: u64 = 6;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub ed25519: ed25519::AppPublic,
		pub sr25519: sr25519::AppPublic,
		pub ecdsa: ecdsa::AppPublic,
	}
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			version()
		}

		fn execute_block(block: Block) {
			log::trace!(target: LOG_TARGET, "execute_block: {block:#?}");
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			log::trace!(target: LOG_TARGET, "initialize_block: {header:#?}");
			Executive::initialize_block(header);
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			unimplemented!()
		}

		fn metadata_at_version(_version: u32) -> Option<OpaqueMetadata> {
			unimplemented!()
		}
		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			unimplemented!()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			log::trace!(target: LOG_TARGET, "finalize_block");
			Executive::finalize_block()
		}

		fn inherent_extrinsics(_data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			vec![]
		}

		fn check_inherents(_block: Block, _data: InherentData) -> CheckInherentsResult {
			CheckInherentsResult::new()
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl self::TestAPI<Block> for Runtime {
		fn test_bls12_381_g1_mul_projective_crypto(base: Vec<u8>, scalar: Vec<u8>) -> Vec<u8> {
			sp_crypto_ec_utils::elliptic_curves::bls12_381_mul_projective_g1(base, scalar)
				.expect("Projective mul works for g1 in bls12_381")
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(1000)
		}

		fn authorities() -> Vec<AuraId> {
			SubstrateTest::authorities().into_iter().map(|auth| AuraId::from(auth)).collect()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(_: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(None)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
			Vec::new()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			0
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_consensus_grandpa::EquivocationProof<
			<Block as BlockT>::Hash,
			NumberFor<Block>,
			>,
			_key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			_authority_id: sp_consensus_grandpa::AuthorityId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			None
		}
	}
}
