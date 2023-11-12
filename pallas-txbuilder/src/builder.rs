use std::{collections::HashMap, time::Instant};

use pallas_primitives::babbage::{
    AddrKeyhash, Certificate, ExUnits, NativeScript, NetworkId, PlutusData, PlutusV1Script,
    PlutusV2Script, Redeemer, RedeemerTag, RewardAccount, TransactionBody, TransactionInput,
    TransactionOutput, WitnessSet,
};

use pallas_crypto::hash::Hash;

use crate::{
    asset::MultiAsset,
    plutus_script::RedeemerPurpose,
    transaction::{self, OutputExt},
    util::*,
    NetworkParams, ValidationError,
};

pub struct TransactionBuilder {
    network_params: NetworkParams,

    inputs: HashMap<TransactionInput, Option<TransactionOutput>>,
    outputs: Vec<TransactionOutput>,
    reference_inputs: HashMap<TransactionInput, Option<TransactionOutput>>,
    fee: Option<u64>,
    collateral: HashMap<TransactionInput, Option<TransactionOutput>>,
    collateral_return: Option<TransactionOutput>,
    mint: Option<MultiAsset<i64>>,
    valid_from_slot: Option<u64>,
    invalid_from_slot: Option<u64>,
    withdrawals: HashMap<RewardAccount, u64>,
    certificates: Vec<Certificate>,
    required_signers: Vec<AddrKeyhash>,
    network_id: Option<u32>,
    native_scripts: Vec<NativeScript>,
    plutus_v1_scripts: Vec<PlutusV1Script>,
    plutus_v2_scripts: Vec<PlutusV2Script>,
    plutus_data: Vec<PlutusData>,
    redeemers: HashMap<RedeemerPurpose, (PlutusData, ExUnits)>,
    script_data_hash: Option<Hash<32>>,
}

impl TransactionBuilder {
    pub fn new(network_params: NetworkParams) -> TransactionBuilder {
        TransactionBuilder {
            network_params,

            // .. Default::default() // TODO,
            inputs: Default::default(),
            outputs: Default::default(),
            reference_inputs: Default::default(),
            fee: Default::default(),
            collateral: Default::default(),
            collateral_return: Default::default(),
            mint: Default::default(),
            valid_from_slot: Default::default(),
            invalid_from_slot: Default::default(),
            withdrawals: Default::default(),
            certificates: Default::default(),
            required_signers: Default::default(),
            network_id: Default::default(),
            native_scripts: Default::default(),
            plutus_v1_scripts: Default::default(),
            plutus_v2_scripts: Default::default(),
            plutus_data: Default::default(),
            redeemers: Default::default(),
            script_data_hash: Default::default(),
        }
    }

    pub fn input(mut self, input: TransactionInput, resolved: Option<TransactionOutput>) -> Self {
        self.inputs.insert(input, resolved);
        self
    }

    pub fn reference_input(
        mut self,
        input: TransactionInput,
        resolved: Option<TransactionOutput>,
    ) -> Self {
        self.reference_inputs.insert(input, resolved);
        self
    }

    pub fn fee(mut self, fee: u64) -> Self {
        self.fee = Some(fee);
        self
    }

    pub fn collateral(
        mut self,
        input: TransactionInput,
        resolved: Option<TransactionOutput>,
    ) -> Self {
        self.collateral.insert(input, resolved);
        self
    }

    pub fn collateral_return(mut self, output: TransactionOutput) -> Self {
        self.collateral_return = Some(output);
        self
    }

    pub fn output(mut self, output: TransactionOutput) -> Self {
        self.outputs.push(output);
        self
    }

    pub fn mint(mut self, assets: MultiAsset<i64>) -> Self {
        self.mint = Some(assets);

        self
    }

    pub fn require_signer(mut self, signer: AddrKeyhash) -> Self {
        self.required_signers.push(signer);
        self
    }

    pub fn network_id(mut self, nid: u32) -> Self {
        self.network_id = Some(nid);
        self
    }

    pub fn valid_from(mut self, timestamp: Instant) -> Result<Self, ValidationError> {
        self.valid_from_slot = Some(
            self.network_params
                .timestamp_to_slot(timestamp)
                .ok_or(ValidationError::InvalidTimestamp)?,
        );

        Ok(self)
    }

    pub fn valid_from_slot(mut self, slot: u64) -> Self {
        self.valid_from_slot = Some(slot);
        self
    }

    pub fn invalid_from(mut self, timestamp: Instant) -> Result<Self, ValidationError> {
        self.invalid_from_slot = Some(
            self.network_params
                .timestamp_to_slot(timestamp)
                .ok_or(ValidationError::InvalidTimestamp)?,
        );

        Ok(self)
    }

    pub fn invalid_from_slot(mut self, slot: u64) -> Self {
        self.invalid_from_slot = Some(slot);
        self
    }

    pub fn withdrawal(mut self, account: RewardAccount, amount: u64) -> Self {
        self.withdrawals.insert(account, amount);
        self
    }

    pub fn certificate(mut self, cert: Certificate) -> Self {
        self.certificates.push(cert);
        self
    }

    /// Add a native script to the transaction
    ///
    /// You can use NativeScriptBuilder to create the NativeScript object type
    pub fn native_script(mut self, script: NativeScript) -> Self {
        self.native_scripts.push(script);
        self
    }

    pub fn plutus_v1_script(mut self, script: PlutusV1Script) -> Self {
        self.plutus_v1_scripts.push(script);
        self
    }

    pub fn plutus_v2_script(mut self, script: PlutusV2Script) -> Self {
        self.plutus_v2_scripts.push(script);
        self
    }

    pub fn plutus_data(mut self, data: impl Into<PlutusData>) -> Self {
        self.plutus_data.push(data.into());
        self
    }

    pub fn redeemer(
        mut self,
        redeemer: RedeemerPurpose,
        data: PlutusData,
        ex_units: ExUnits,
    ) -> Self {
        self.redeemers.insert(redeemer, (data, ex_units));
        self
    }

    pub fn script_data_hash(mut self, hash: Hash<32>) -> Self {
        self.script_data_hash = Some(hash);
        self
    }

    pub fn build(self) -> Result<transaction::Transaction, ValidationError> {
        if self.inputs.is_empty() {
            return Err(ValidationError::NoInputs);
        }

        if self
            .collateral_return
            .as_ref()
            .map(|i| i.is_multiasset())
            .unwrap_or(false)
        {
            return Err(ValidationError::InvalidCollateralReturn);
        }

        let mut inputs = self
            .inputs
            .iter()
            .map(|(k, _)| k.clone())
            .collect::<Vec<_>>();
        inputs.sort_unstable_by_key(|x| (x.transaction_id, x.index));

        let reference_inputs = self
            .reference_inputs
            .iter()
            .map(|(k, _)| k.clone())
            .collect();

        let collaterals = self.collateral.iter().map(|(k, _)| k.clone()).collect();

        let outputs = self.outputs.clone();

        let mint = self.mint.map(|x| x.build());

        let mut mint_policies = mint
            .clone()
            .unwrap_or(vec![].into())
            .iter()
            .map(|(p, _)| *p)
            .collect::<Vec<_>>();
        mint_policies.sort_unstable_by_key(|x| *x);

        let mut redeemers = vec![];

        for (rp, (data, ex_units)) in self.redeemers {
            match rp {
                RedeemerPurpose::Spend(ref txin) => {
                    let index = inputs
                        .iter()
                        .position(|x| x == txin)
                        .ok_or(ValidationError::RedeemerPurposeMissing(rp))?
                        as u32;

                    redeemers.push(Redeemer {
                        tag: RedeemerTag::Spend,
                        index,
                        data,
                        ex_units,
                    })
                }
                RedeemerPurpose::Mint(pid) => {
                    let index = mint_policies
                        .iter()
                        .position(|x| *x == pid)
                        .ok_or(ValidationError::RedeemerPurposeMissing(rp))?
                        as u32;

                    redeemers.push(Redeemer {
                        tag: RedeemerTag::Mint,
                        index,
                        data,
                        ex_units,
                    })
                }
                _ => todo!(), // TODO: reward, cert
            }
        }

        /*
            TODO: script data hash computation (requires resolved utxos)

            let buf = vec![];
            let mut script_hash_data = Encoder::new(buf);
            if !self.plutus_data.is_empty() && redeemers.is_empty() {
                script_hash_data.array(0).unwrap(); // TODO

                script_hash_data.array(self.plutus_data.len() as u64).unwrap();
                for pd in self.plutus_data.iter() {
                    script_hash_data.encode(pd).unwrap();
                }

                script_hash_data.map(0).unwrap();
            } else {
                script_hash_data.array(redeemers.len() as u64).unwrap();
                for rdmr in redeemers.iter() {
                    script_hash_data.encode(rdmr).unwrap();
                }

                script_hash_data.array(self.plutus_data.len() as u64).unwrap();
                for pd in self.plutus_data.iter() {
                    script_hash_data.encode(pd).unwrap();
                }

                // TODO: cost models
            }
        */

        let mut tx = transaction::Transaction {
            body: TransactionBody {
                inputs,
                outputs,
                ttl: self.invalid_from_slot,
                validity_interval_start: self.valid_from_slot,
                fee: self.fee.unwrap_or_default(), // TODO
                certificates: opt_if_empty(self.certificates),
                withdrawals: None, // TODO
                update: None,      // TODO
                auxiliary_data_hash: None,
                mint,
                script_data_hash: self.script_data_hash,
                collateral: opt_if_empty(collaterals),
                required_signers: opt_if_empty(self.required_signers),
                network_id: NetworkId::from_u64(self.network_params.network_id()),
                collateral_return: self.collateral_return,
                total_collateral: None, // TODO
                reference_inputs: opt_if_empty(reference_inputs),
            },
            witness_set: WitnessSet {
                vkeywitness: None,
                native_script: opt_if_empty(self.native_scripts),
                bootstrap_witness: None,
                plutus_v1_script: opt_if_empty(self.plutus_v1_scripts),
                plutus_v2_script: opt_if_empty(self.plutus_v2_scripts),
                plutus_data: opt_if_empty(self.plutus_data),
                redeemer: opt_if_empty(redeemers),
            },
            is_valid: true,       // TODO
            auxiliary_data: None, // TODO
        };

        tx.body.auxiliary_data_hash = tx.auxiliary_data.clone().map(hash_to_bytes);

        Ok(tx)
    }

    pub fn build_hex(self) -> Result<String, ValidationError> {
        Ok(self.build()?.hex_encoded()?)
    }
}
