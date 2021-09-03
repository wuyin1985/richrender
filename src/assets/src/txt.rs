use type_uuid::TypeUuid;
use distill::{
    core::{AssetUuid},
    importer::{Error, ImportOp, ImportedAsset, Importer, ImporterValue, Result},
};
use std::io::Read;
use serde::{Serialize, Deserialize};


#[derive(TypeUuid, Serialize, Deserialize, Debug)]
#[uuid = "25f4bbd9-f0c4-4e5e-bfda-dffb26a57b1d"]
pub struct Txt {
    data: String,
}


pub struct TxtImporter;

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "860f9ba6-8956-45d7-9a77-17730c8cfffb"]
pub struct SimpleState(Option<AssetUuid>);

impl Importer for TxtImporter {
    fn version_static() -> u32 where Self: Sized {
        1
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();
    type State = SimpleState;

    fn import(&self, op: &mut ImportOp, source: &mut dyn Read, options: &Self::Options, state: &mut Self::State) -> Result<ImporterValue> {
        let id = state.0.unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = SimpleState(Some(id));

        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;
        let data = std::str::from_utf8(&bytes).map_err(|e| Error::Custom(e.to_string()))?;
        let asset = Txt { data: data.to_string() };
        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(asset),
            }]
        })
    }
}