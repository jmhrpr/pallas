use std::collections::HashMap;

use pallas_codec::utils::Bytes;
use pallas_primitives::babbage::PolicyId;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AssetError {
    #[error("Invalid asset name")]
    InvalidAssetName(String),
}

#[derive(Debug, Clone, Default)]
pub struct MultiAsset<T> {
    assets: HashMap<PolicyId, HashMap<Bytes, T>>,
}

impl<T: Default + Copy> MultiAsset<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn from_map(asset_map: HashMap<PolicyId, HashMap<Bytes, T>>) -> Self {
        MultiAsset { assets: asset_map }
    }

    pub fn add(mut self, policy_id: PolicyId, name: &[u8], amount: T) -> Result<Self, AssetError> {
        if name.len() > 32 {
            return Err(AssetError::InvalidAssetName("name max len exceeded".into()));
        }

        self.assets
            .entry(policy_id)
            .or_default()
            .insert(name.into(), amount);

        Ok(self)
    }

    pub(crate) fn build(self) -> pallas_primitives::babbage::Multiasset<T> {
        let assets = self
            .assets
            .into_iter()
            .map(|(policy_id, assets)| (policy_id, assets.into_iter().collect::<Vec<_>>().into()))
            .collect::<Vec<_>>();

        assets.into()
    }
}
