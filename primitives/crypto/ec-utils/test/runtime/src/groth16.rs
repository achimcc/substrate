#![warn(unused)]
#![deny(
	trivial_casts,
	trivial_numeric_casts,
	variant_size_differences,
	stable_features,
	non_shorthand_field_patterns,
	renamed_and_removed_lints,
	private_in_public,
	unsafe_code
)]

// For randomness (during paramgen and proof generation)
use ark_ec::pairing::Pairing;
use ark_groth16::Groth16;
use ark_snark::{CircuitSpecificSetupSNARK, SNARK};
use ark_std::{
	rand::{Rng, RngCore, SeedableRng},
	vec::Vec,
};
use sp_ark_bls12_381::Fr;

// For benchmarking
// use std::time::{Duration, Instant};

// Bring in some tools for using pairing-friendly curves
// We're going to use the BLS12-377 pairing-friendly elliptic curve.
use ark_ff::Field;
use ark_std::test_rng;
use sp_crypto_ec_utils::bls12_377::Bls12_377;

// We'll use these interfaces to construct our circuit.
use ark_relations::{
	lc, ns,
	r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable},
};

const MIMC_ROUNDS: usize = 3022;

/// This is an implementation of MiMC, specifically a
/// variant named `LongsightF322p3` for BLS12-377.
/// See http://eprint.iacr.org/2016/492 for more
/// information about this construction.
///
/// ```
/// function LongsightF322p3(xL ⦂ Fp, xR ⦂ Fp) {
///     for i from 0 up to 321 {
///         xL, xR := xR + (xL + Ci)^3, xL
///     }
///     return xL
/// }
/// ```
fn mimc<F: Field>(mut xl: F, mut xr: F, constants: &[F]) -> F {
	assert_eq!(constants.len(), MIMC_ROUNDS);

	for i in 0..MIMC_ROUNDS {
		let mut tmp1 = xl;
		tmp1.add_assign(&constants[i]);
		let mut tmp2 = tmp1;
		tmp2.square_in_place();
		tmp2.mul_assign(&tmp1);
		tmp2.add_assign(&xr);
		xr = xl;
		xl = tmp2;
	}

	xl
}

/// This is our demo circuit for proving knowledge of the
/// preimage of a MiMC hash invocation.
struct MiMCDemo<'a, F: Field> {
	xl: Option<F>,
	xr: Option<F>,
	constants: &'a [F],
}

/// Our demo circuit implements this `Circuit` trait which
/// is used during paramgen and proving in order to
/// synthesize the constraint system.
impl<'a, F: Field> ConstraintSynthesizer<F> for MiMCDemo<'a, F> {
	fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
		assert_eq!(self.constants.len(), MIMC_ROUNDS);

		// Allocate the first component of the preimage.
		let mut xl_value = self.xl;
		let mut xl =
			cs.new_witness_variable(|| xl_value.ok_or(SynthesisError::AssignmentMissing))?;

		// Allocate the second component of the preimage.
		let mut xr_value = self.xr;
		let mut xr =
			cs.new_witness_variable(|| xr_value.ok_or(SynthesisError::AssignmentMissing))?;

		for i in 0..MIMC_ROUNDS {
			// xL, xR := xR + (xL + Ci)^3, xL
			let ns = ns!(cs, "round");
			let cs = ns.cs();

			// tmp = (xL + Ci)^2
			let tmp_value = xl_value.map(|mut e| {
				e.add_assign(&self.constants[i]);
				e.square_in_place();
				e
			});
			let tmp =
				cs.new_witness_variable(|| tmp_value.ok_or(SynthesisError::AssignmentMissing))?;

			cs.enforce_constraint(
				lc!() + xl + (self.constants[i], Variable::One),
				lc!() + xl + (self.constants[i], Variable::One),
				lc!() + tmp,
			)?;

			// new_xL = xR + (xL + Ci)^3
			// new_xL = xR + tmp * (xL + Ci)
			// new_xL - xR = tmp * (xL + Ci)
			let new_xl_value = xl_value.map(|mut e| {
				e.add_assign(&self.constants[i]);
				e.mul_assign(&tmp_value.unwrap());
				e.add_assign(&xr_value.unwrap());
				e
			});

			let new_xl = if i == (MIMC_ROUNDS - 1) {
				// This is the last round, xL is our image and so
				// we allocate a public input.
				cs.new_input_variable(|| new_xl_value.ok_or(SynthesisError::AssignmentMissing))?
			} else {
				cs.new_witness_variable(|| new_xl_value.ok_or(SynthesisError::AssignmentMissing))?
			};

			cs.enforce_constraint(
				lc!() + tmp,
				lc!() + xl + (self.constants[i], Variable::One),
				lc!() + new_xl - xr,
			)?;

			// xR = xL
			xr = xl;
			xr_value = xl_value;

			// xL = new_xL
			xl = new_xl;
			xl_value = new_xl_value;
		}

		Ok(())
	}
}

pub fn test_mimc_groth16<E: Pairing>() {
	// Result<crate::Groth16Ok, crate::Groth16Error> {
	// We're going to use the Groth16 proving system
	// This may not be cryptographically safe, use
	// `OsRng` (for example) in production software.
	let mut rng = ark_std::rand::rngs::StdRng::seed_from_u64(test_rng().next_u64());

	// Generate the MiMC round constants
	let constants = (0..MIMC_ROUNDS).map(|_| rng.gen()).collect::<Vec<_>>();

	// Create parameters for our circuit
	let (pk, vk) = {
		let c = MiMCDemo::<Fr> { xl: None, xr: None, constants: &constants };

		Groth16::<E>::setup(c, &mut rng).unwrap()
	};

	let pvk = Groth16::<E>::process_vk(&vk).unwrap();

	// Generate a random preimage and compute the image
	let xl = rng.gen();
	let xr = rng.gen();
	let image = mimc(xl, xr, &constants);
	let c = MiMCDemo { xl: Some(xl), xr: Some(xr), constants: &constants };

	// Create a groth16 proof with our parameters.
	let proof = Groth16::<E>::prove(&pk, c, &mut rng).unwrap();
	Groth16::<E>::verify_with_processed_vk(&pvk, &[image], &proof).unwrap();
}