# Witness

anoncreds-rs does validate for witness value if revocation registry and revocation registry id is present:

https://github.com/rngadam/anoncreds-rs/blob/44a22edc49275f4ad0ea805ae9b033d86c22b405/src/data_types/credential.rs#L64-L66

## Proof

Prover recomputes witness in the function create_or_update_revocation_state:

https://github.com/rngadam/anoncreds-rs/blob/44a22edc49275f4ad0ea805ae9b033d86c22b405/src/services/prover.rs#L577
https://github.com/rngadam/anoncreds-rs/blob/44a22edc49275f4ad0ea805ae9b033d86c22b405/src/services/prover.rs#L607-L635

## Issuance

the witness is created using CLCredentialIssuer:

https://github.com/rngadam/anoncreds-rs/blob/44a22edc49275f4ad0ea805ae9b033d86c22b405/src/services/issuer.rs#L716-L722

which in turns calls Issuer::sign_credential_with_revoc:

https://github.com/rngadam/anoncreds-rs/blob/44a22edc49275f4ad0ea805ae9b033d86c22b405/src/services/issuer.rs#L814-L829

which calls _new_non_revocation_credential in the repo anoncreds-clsignatures-rs

https://github.com/hyperledger/anoncreds-clsignatures-rs/blob/61b2acea99e6da6d06ab83c117d7a3605fe79322/src/issuer.rs#L1297C1-L1297C39

## Revocable vs non-revocable 

in CredentialSignatureProofValue, witness may be none as well as rev_reg_id and rev_reg (for credentials that are not revocable)

https://github.com/rngadam/anoncreds-rs/blob/44a22edc49275f4ad0ea805ae9b033d86c22b405/src/data_types/w3c/proof.rs#L263-L275
