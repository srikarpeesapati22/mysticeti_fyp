// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use digest::Digest;
use pqcrypto_mldsa::mldsa44;
use pqcrypto_mldsa::mldsa44::PublicKey as PublicKeyExternal;
use pqcrypto_traits::sign::{SecretKey, VerificationError};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use zeroize::Zeroize;
#[cfg(not(test))]
use pqcrypto_traits::sign::DetachedSignature;

#[cfg(not(test))]
use pqcrypto_traits::sign::SecretKey as SecretKeyExternal;

#[cfg(not(test))]
use pqcrypto_traits::sign::PublicKey as PublicKeyExternal2;

#[cfg(not(test))]
use crate::types::Vote;
use crate::{
    serde::{ByteRepr, BytesVisitor},
    types::{
        AuthorityIndex, BaseStatement, BlockReference, EpochStatus, RoundNumber, StatementBlock,
        TimestampNs,
    },
};

//pub const SIGNATURE_SIZE: usize = 64;
pub const SIGNATURE_SIZE: usize = mldsa44::signature_bytes();
//pub const PUBLIC_KEY_SIZE: usize = mldsa44::public_key_bytes();
pub const SECRET_KEY_SIZE: usize = mldsa44::secret_key_bytes();
pub const BLOCK_DIGEST_SIZE: usize = 32;

#[derive(Clone, Copy, Eq, Ord, PartialOrd, PartialEq, Default, Hash)]
pub struct BlockDigest([u8; BLOCK_DIGEST_SIZE]);

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize)]
//pub struct PublicKey(ed25519_consensus::VerificationKey);
pub struct PublicKey(PublicKeyExternal);
impl std::cmp::Eq for PublicKey {}
impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey")
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct SecretKeyLocal(mldsa44::SecretKey);
impl Default for SecretKeyLocal {
    fn default() -> Self {
        SecretKeyLocal(SecretKey::from_bytes(&[0u8; SECRET_KEY_SIZE]).unwrap())
    }
}
impl zeroize::DefaultIsZeroes for SecretKeyLocal {}

#[derive(Clone, Copy, Eq, Ord, PartialOrd, PartialEq, Hash)]
pub struct SignatureBytes([u8; SIGNATURE_SIZE]);

// Box ensures value is not copied in memory when Signer itself is moved around for better security
#[derive(Serialize, Deserialize)]
pub struct Signer(Box<SecretKeyLocal>, PublicKey);

#[cfg(not(test))]
type BlockHasher = blake2::Blake2b<digest::consts::U32>;

impl BlockDigest {
    #[cfg(not(test))]
    pub fn new(
        authority: AuthorityIndex,
        round: RoundNumber,
        includes: &[BlockReference],
        statements: &[BaseStatement],
        meta_creation_time_ns: TimestampNs,
        epoch_marker: EpochStatus,
        signature: &SignatureBytes,
    ) -> Self {
        let mut hasher = BlockHasher::default();
        Self::digest_without_signature(
            &mut hasher,
            authority,
            round,
            includes,
            statements,
            meta_creation_time_ns,
            epoch_marker,
        );
        hasher.update(signature);
        Self(hasher.finalize().into())
    }

    #[cfg(test)]
    pub fn new(
        _authority: AuthorityIndex,
        _round: RoundNumber,
        _includes: &[BlockReference],
        _statements: &[BaseStatement],
        _meta_creation_time_ns: TimestampNs,
        _epoch_marker: EpochStatus,
        _signature: &SignatureBytes,
    ) -> Self {
        Default::default()
    }

    /// There is a bit of a complexity around what is considered block digest and what is being signed
    ///
    /// * Block signature covers all the fields in the block, except for signature and reference.digest
    /// * Block digest(e.g. block.reference.digest) covers all the above **and** block signature
    ///
    /// This is not very beautiful, but it allows to optimize block synchronization,
    /// by skipping signature verification for all the descendants of the certified block.
    #[cfg(not(test))]
    fn digest_without_signature(
        hasher: &mut BlockHasher,
        authority: AuthorityIndex,
        round: RoundNumber,
        includes: &[BlockReference],
        statements: &[BaseStatement],
        meta_creation_time_ns: TimestampNs,
        epoch_marker: EpochStatus,
    ) {
        authority.crypto_hash(hasher);
        round.crypto_hash(hasher);
        for include in includes {
            include.crypto_hash(hasher);
        }
        for statement in statements {
            match statement {
                BaseStatement::Share(tx) => {
                    [0].crypto_hash(hasher);
                    tx.crypto_hash(hasher);
                }
                BaseStatement::Vote(id, Vote::Accept) => {
                    [1].crypto_hash(hasher);
                    id.crypto_hash(hasher);
                }
                BaseStatement::Vote(id, Vote::Reject(None)) => {
                    [2].crypto_hash(hasher);
                    id.crypto_hash(hasher);
                }
                BaseStatement::Vote(id, Vote::Reject(Some(other))) => {
                    [3].crypto_hash(hasher);
                    id.crypto_hash(hasher);
                    other.crypto_hash(hasher);
                }
                BaseStatement::VoteRange(range) => {
                    [4].crypto_hash(hasher);
                    range.crypto_hash(hasher);
                }
            }
        }
        meta_creation_time_ns.crypto_hash(hasher);
        epoch_marker.crypto_hash(hasher);
    }
}

pub trait AsBytes {
    // This is pretty much same as AsRef<[u8]>
    //
    // We need this separate trait because we want to impl CryptoHash
    // for primitive types(u64, etc) and types like XxxDigest that implement AsRef<[u8]>.
    //
    // Rust unfortunately does not allow to impl trait for AsRef<[u8]> and primitive types like u64.
    //
    // While AsRef<[u8]> is not implemented for u64, it seem to be reserved in compiler,
    // so `impl CryptoHash for u64` and `impl<T: AsRef<[u8]>> CryptoHash for T` collide.
    fn as_bytes(&self) -> &[u8];
}

impl<const N: usize> AsBytes for [u8; N] {
    fn as_bytes(&self) -> &[u8] {
        self
    }
}

pub trait CryptoHash {
    fn crypto_hash(&self, state: &mut impl Digest);
}

impl CryptoHash for u64 {
    fn crypto_hash(&self, state: &mut impl Digest) {
        state.update(self.to_be_bytes());
    }
}

impl CryptoHash for u128 {
    fn crypto_hash(&self, state: &mut impl Digest) {
        state.update(self.to_be_bytes());
    }
}

impl<T: AsBytes> CryptoHash for T {
    fn crypto_hash(&self, state: &mut impl Digest) {
        state.update(self.as_bytes());
    }
}

impl PublicKey {
    #[cfg(not(test))]
    pub fn verify_block(&self, block: &StatementBlock) -> Result<(), VerificationError> {
        use pqcrypto_traits::sign::PublicKey;

        let signature: &[u8] = &block.signature().0;
        let detached_signature = mldsa44::DetachedSignature::from_bytes(signature).map_err(|_| VerificationError::UnknownVerificationError)?;
        //let signature = mldsa44::DetachedSignature::from_bytes(&block.signature().0);
        let mut hasher = BlockHasher::default();
        BlockDigest::digest_without_signature(
            &mut hasher,
            block.author(),
            block.round(),
            block.includes(),
            block.statements(),
            block.meta_creation_time_ns(),
            block.epoch_changed(),
        );
        let digest: [u8; BLOCK_DIGEST_SIZE] = hasher.finalize().into();
        let pub_key_bytes: &[u8] = mldsa44::PublicKey::as_bytes(&self.0);
        let pub_key: PublicKeyExternal = mldsa44::PublicKey::from_bytes(&pub_key_bytes).map_err(|_| VerificationError::UnknownVerificationError)?;
        //mldsa44::verify_detached_signature(&detached_signature, digest.as_ref(), &pub_key).map_err(|_| VerificationError::InvalidSignature)
        println!("Public Key on Verification: {:?}\nSignature on Verification: {:?}", PublicKeyExternal::as_bytes(&self.0), DetachedSignature::as_bytes(&detached_signature));
        mldsa44::verify_detached_signature(&detached_signature, digest.as_ref(), &pub_key)

    }

    pub fn as_bytes_2(&self) -> &[u8] {
        use pqcrypto_traits::sign::PublicKey as PublicKeyExternal2;

        PublicKeyExternal::as_bytes(&self.0)
    }

    #[cfg(test)]
    pub fn verify_block(&self, _block: &StatementBlock) -> Result<(), VerificationError> {
        Ok(())
    }
}

impl Signer {
    pub fn new() -> Signer {
        let keypair = mldsa44::keypair();
        let public_key_local = PublicKey(keypair.0);
        println!("Public Key on Generation: {:?}\n", PublicKey::as_bytes_2(&public_key_local));
        let secret_key_local = Box::new(SecretKeyLocal(keypair.1));

        Signer {
            0: secret_key_local,
            1: public_key_local,
        }
    }

    pub fn new_for_test(n: usize) -> Vec<Self> {
        //let mut rng = StdRng::seed_from_u64(0);
        (0..n).map(|_| Signer::new()).collect()
    }

    #[cfg(not(test))]
    pub fn sign_block(
        &self,
        authority: AuthorityIndex,
        round: RoundNumber,
        includes: &[BlockReference],
        statements: &[BaseStatement],
        meta_creation_time_ns: TimestampNs,
        epoch_marker: EpochStatus,
    ) -> SignatureBytes {
        let mut hasher = BlockHasher::default();
        BlockDigest::digest_without_signature(
            &mut hasher,
            authority,
            round,
            includes,
            statements,
            meta_creation_time_ns,
            epoch_marker,
        );
        let digest: [u8; BLOCK_DIGEST_SIZE] = hasher.finalize().into();
        let signature = mldsa44::detached_sign(&digest, &self.0.0);
        let signature_bytes = mldsa44::DetachedSignature::as_bytes(&signature);
        let s_bytes: [u8; SIGNATURE_SIZE] = signature_bytes.try_into().expect("Signature must be 2420 bytes");
        //assert!(false, "Public Key: {:?}, Private Key: {:?}, Signature: {:?}", PublicKeyExternal::as_bytes(&self.1.0), mldsa44::SecretKey::as_bytes(&self.0.0), mldsa44::DetachedSignature::as_bytes(&signature));
        assert!(mldsa44::verify_detached_signature(&mldsa44::DetachedSignature::from_bytes(&SignatureBytes(s_bytes).0).unwrap(), digest.as_ref(), &self.public_key().0).is_ok(), "Verification Failed.");
        println!("Public Key on Signing: {:?}\nDetached Signature at Signing: {:?}\nSignature Bytes at Signing: {:?}", PublicKey::as_bytes_2(&self.1), &DetachedSignature::as_bytes(&signature), &SignatureBytes(s_bytes).0);
        SignatureBytes(s_bytes)
        //SignatureBytes(*<&[u8; SIGNATURE_SIZE]>::try_from(signature.as_bytes()).unwrap())
    }

    #[cfg(test)]
    pub fn sign_block(
        &self,
        _authority: AuthorityIndex,
        _round: RoundNumber,
        _includes: &[BlockReference],
        _statements: &[BaseStatement],
        _meta_creation_time_ns: TimestampNs,
        _epoch_marker: EpochStatus,
    ) -> SignatureBytes {
        Default::default()
    }

    pub fn public_key(&self) -> PublicKey {
        self.1
    }
}

impl AsRef<[u8]> for BlockDigest {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for SignatureBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsBytes for BlockDigest {
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl AsBytes for SignatureBytes {
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for BlockDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", hex::encode(self.0))
    }
}

impl fmt::Display for BlockDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", hex::encode(&self.0[..4]))
    }
}

impl fmt::Debug for Signer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signer(public_key={:?})", self.public_key())
    }
}

impl fmt::Display for Signer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signer(public_key={:?})", self.public_key())
    }
}

impl Default for SignatureBytes {
    fn default() -> Self {
        Self([0u8; SIGNATURE_SIZE])
    }
}

impl ByteRepr for SignatureBytes {
    fn try_copy_from_slice<E: de::Error>(v: &[u8]) -> Result<Self, E> {
        if v.len() != SIGNATURE_SIZE {
            return Err(E::custom(format!("Invalid signature length: {}", v.len())));
        }
        let mut inner = [0u8; SIGNATURE_SIZE];
        inner.copy_from_slice(v);
        Ok(Self(inner))
    }
}

impl Serialize for SignatureBytes {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for SignatureBytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_bytes(BytesVisitor::new())
    }
}

impl ByteRepr for BlockDigest {
    fn try_copy_from_slice<E: de::Error>(v: &[u8]) -> Result<Self, E> {
        if v.len() != BLOCK_DIGEST_SIZE {
            return Err(E::custom(format!(
                "Invalid block digest length: {}",
                v.len()
            )));
        }
        let mut inner = [0u8; BLOCK_DIGEST_SIZE];
        inner.copy_from_slice(v);
        Ok(Self(inner))
    }
}

impl Serialize for BlockDigest {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for BlockDigest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_bytes(BytesVisitor::new())
    }
}

impl Drop for Signer {
    fn drop(&mut self) {
        self.0.zeroize()
    }
}

pub fn dummy_signer() -> Signer {
    Signer::new()
}

pub fn dummy_public_key() -> PublicKey {
    dummy_signer().public_key()
}
