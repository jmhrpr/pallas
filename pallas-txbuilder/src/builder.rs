use pallas_primitives::babbage::{
    AddrKeyhash, Certificate, NetworkId, TransactionBody, TransactionInput, TransactionOutput,
    WitnessSet,
};

use crate::{
    fee::Fee, prelude::MultiAsset, strategy::*, transaction, NetworkParams, ValidationError,
};

pub struct TransactionBuilder<T> {
    strategy: T,

    network_params: NetworkParams,
    mint: Option<MultiAsset<i64>>,
    required_signers: Vec<AddrKeyhash>,
    valid_after: Option<u64>,
    valid_until: Option<u64>,

    certificates: Vec<Certificate>,
}

impl<T: Default> Default for TransactionBuilder<T> {
    fn default() -> Self {
        Self {
            network_params: NetworkParams::mainnet(),

            strategy: Default::default(),
            mint: Default::default(),
            required_signers: Default::default(),
            valid_after: Default::default(),
            valid_until: Default::default(),
            certificates: Default::default(),
        }
    }
}

impl<T: Default + Strategy> TransactionBuilder<T> {
    pub fn new(network_params: NetworkParams) -> TransactionBuilder<T> {
        TransactionBuilder {
            network_params,
            ..Default::default()
        }
    }

    pub fn mint(mut self, assets: MultiAsset<i64>) -> Self {
        self.mint = Some(assets);

        self
    }

    pub fn require_signer(mut self, signer: AddrKeyhash) -> Self {
        self.required_signers.push(signer);
        self
    }

    pub fn valid_after(mut self, timestamp: u64) -> Self {
        self.valid_after = Some(timestamp);
        self
    }

    pub fn valid_until(mut self, timestamp: u64) -> Self {
        self.valid_until = Some(timestamp);
        self
    }

    pub fn certificate(mut self, cert: Certificate) -> Self {
        self.certificates.push(cert);
        self
    }

    pub fn build(self) -> Result<transaction::Transaction, ValidationError> {
        let (inputs, outputs) = self.strategy.resolve()?;
        let mut tx = transaction::Transaction {
            body: TransactionBody {
                inputs,
                outputs,
                ttl: self.convert_timestamp(self.valid_until)?,
                validity_interval_start: self.convert_timestamp(self.valid_after)?,
                fee: 0,
                certificates: opt_if_empty(self.certificates),
                withdrawals: None,
                update: None,
                auxiliary_data_hash: None,
                mint: self.mint.map(|x| x.build()),
                script_data_hash: None,
                collateral: None,
                required_signers: opt_if_empty(self.required_signers),
                network_id: NetworkId::from_u64(self.network_params.network_id()),
                collateral_return: None,
                total_collateral: None,
                reference_inputs: None,
            },
            witness_set: WitnessSet {
                vkeywitness: None,
                native_script: None,
                bootstrap_witness: None,
                plutus_v1_script: None,
                plutus_data: None,
                redeemer: None,
                plutus_v2_script: None,
            },
            is_valid: true,
            auxiliary_data: None,
        };

        tx.body.fee = Fee::linear().calculate(&tx)?;

        Ok(tx)
    }

    fn convert_timestamp(&self, timestamp: Option<u64>) -> Result<Option<u64>, ValidationError> {
        match timestamp {
            Some(v) => match self.network_params.timestamp_to_slot(v) {
                Some(v) => Ok(Some(v)),
                None => return Err(ValidationError::InvalidTimestamp),
            },
            _ => Ok(None),
        }
    }
}

impl TransactionBuilder<Manual> {
    pub fn input(mut self, input: TransactionInput, resolved: TransactionOutput) -> Self {
        self.strategy.input(input, resolved);
        self
    }

    pub fn output(mut self, output: TransactionOutput) -> Self {
        self.strategy.output(output);
        self
    }
}

#[inline(always)]
fn opt_if_empty<T>(v: Vec<T>) -> Option<Vec<T>> {
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}
