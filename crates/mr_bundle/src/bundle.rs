use bytes::Bytes;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    encode,
    error::{MrBundleError, MrBundleResult},
    location::Location,
    manifest::Manifest,
    resource::Resource,
};
use std::{collections::HashMap, convert::TryFrom};

#[derive(Serialize, Deserialize, derive_more::From, derive_more::AsRef)]
pub struct Blob(#[serde(with = "serde_bytes")] Vec<u8>);

/// A Manifest bundled together, optionally, with the Resources that it describes.
/// This is meant to be serialized for standalone distribution, and deserialized
/// by the receiver.
///
/// The manifest may describe locations of resources not included in the Bundle.
//
// TODO: make clear the difference between the partial set of resources and
// the full set of resolved resources.
//
// NB: It would be so nice if this were Deserializable, but there are problems
// with using the derive macro here.
#[derive(Debug, PartialEq, Eq)]
pub struct Bundle<M, R>
where
    M: Manifest,
    R: Resource,
{
    manifest: M,
    resources: HashMap<Location, R>,
}

#[derive(Serialize, Deserialize)]
struct BundleSerialized {
    manifest: Vec<u8>,
    resources: Vec<u8>,
}

impl<M, R> TryFrom<&Bundle<M, R>> for BundleSerialized
where
    M: Manifest,
    R: Resource,
{
    type Error = MrBundleError;
    fn try_from(bundle: &Bundle<M, R>) -> MrBundleResult<BundleSerialized> {
        Ok(Self {
            manifest: crate::encode(&bundle.manifest)?,
            resources: crate::encode(&bundle.resources)?,
        })
    }
}

impl<M, R> TryFrom<&BundleSerialized> for Bundle<M, R>
where
    M: Manifest,
    R: Resource,
{
    type Error = MrBundleError;
    fn try_from(bundle: &BundleSerialized) -> MrBundleResult<Bundle<M, R>> {
        Ok(Self {
            manifest: crate::decode(&bundle.manifest)?,
            resources: crate::decode(&bundle.resources)?,
        })
    }
}

impl<M, R> Bundle<M, R>
where
    M: Manifest,
    R: Resource,
{
    // TODO: break this up into `resolve_all` and `new` which takes a partial set of
    // bundled resources
    pub async fn from_manifest(manifest: M) -> MrBundleResult<Self> {
        let resources: HashMap<Location, R> =
            futures::future::join_all(manifest.locations().into_iter().map(|loc| async move {
                Ok((
                    loc.clone(),
                    crate::decode(&loc.resolve().await?.into_iter().collect::<Vec<u8>>())?,
                ))
            }))
            .await
            .into_iter()
            .collect::<MrBundleResult<HashMap<_, _>>>()?;

        Ok(Self {
            manifest,
            resources,
        })
    }

    pub fn resource(&self, location: &Location) -> Option<&R> {
        self.resources.get(location)
    }

    /// An arbitrary and opaque encoding of the bundle data into a byte array
    // NB: Ideally, Bundle could just implement serde Serialize/Deserialize,
    // but the generic types cause problems
    pub fn encode(&self) -> MrBundleResult<Vec<u8>> {
        crate::encode(&(
            crate::encode(&self.manifest)?,
            crate::encode(&self.resources)?,
        ))
    }

    /// Decode bytes produced by `to_bytes`
    pub fn decode(bytes: &[u8]) -> MrBundleResult<Self> {
        let (m, r): (Vec<u8>, Vec<u8>) = crate::decode(bytes)?;
        Ok(Self {
            manifest: crate::decode(&m)?,
            resources: crate::decode(&r)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_test() {
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(tag = "manifest_version")]
        #[allow(missing_docs)]
        enum Manifest {
            #[serde(rename = "1")]
            #[serde(alias = "\"1\"")]
            V1(ManifestV1),
        }

        impl Manifest for Manifest {
            fn locations(&self) -> Vec<Location> {
                match self {
                    Self::V1(mani) => mani.things.iter().map(|b| b.location.clone()).collect(),
                }
            }
        }

        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct ManifestV1 {
            name: String,
            things: Vec<ThingManifest>,
        }

        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct ThingManifest {
            #[serde(flatten)]
            location: Location,
        }

        #[derive(Clone, Serialize, Deserialize)]
        struct Thing(u32);

        let location1 = Location::Bundled("./1.thing".into());
        let location2 = Location::Bundled("./2.thing".into());

        let manifest = Manifest::V1(ManifestV1 {
            name: "name".to_string(),
            things: vec![
                ThingManifest {
                    location: location1.clone(),
                },
                ThingManifest {
                    location: location2.clone(),
                },
            ],
        });

        let bundle = Bundle {
            manifest,
            resources: maplit::hashmap! {
                location1 => Thing(1),
                location2 => Thing(2),
            },
        };
    }
}