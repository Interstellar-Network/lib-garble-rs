# lib-garble-rs

Replacement for both `api_garble` and `lib_garble`.
Therefore it is splitted into two crates:
- `lib-garble-rs`: contains the Swanky/Fancy-Garbling code related to Garbled Circuits
- `ipfs-client-http-req`: a no_std/sgx compatible basic IPFS client(only for ADD and CAT for now)