# Mysticeti

[![build status](https://img.shields.io/github/actions/workflow/status/asonnino/shamir-bip39/code.yml?branch=main&logo=github&style=flat-square)](https://github.com/asonnino/shamir-bip39/actions)
[![rustc](https://img.shields.io/badge/rustc-1.78+-blue?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![license](https://img.shields.io/badge/license-Apache-blue.svg?style=flat-square)](LICENSE)

The code in this branch is a prototype of Mysticeti. It supplements the paper [Mysticeti: Reaching the Limits of Latency with Uncertified DAGs](https://arxiv.org/abs/2310.14821) enabling reproducible results. 

This repository is forked from the original implementation of Mysticeti.

This paper investigates the integration of post-quantum cryptographic (PQC) signatures into DAG-based consensus protocols, using the Mysticeti protocol as a case study. The primary goal was to replace the existing Elliptic Curve Cryptography digital signature scheme with ML-DSA, a NIST-standardized lattice-based algorithm, to improve resilience against quantum computing threats.

To run/test the PQC version of Mysticeti:

1. Clone this repository

2. Switch to the FIPS_204_Implementation branch.

3. Run ```git submodule update --remote``` in a Terminal of your choice.

4. Run [dryrun.sh](./scripts/dryrun.sh) to run testing.

5. Switching to the main_for_testing branch allows for baseline testing of non-PQC Mysticeti.

## License

This software is licensed as [Apache 2.0](LICENSE).
