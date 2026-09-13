use std::str::FromStr;

use content_addressable::{ContentAddressable, ContentId};

use crate::model::{Artifact, Corpus, MAX_ARTIFACT_BYTES};

impl Artifact {
    pub fn mint(corpus: Corpus) -> Result<Self, String> {
        corpus.validate()?;
        let id = corpus
            .content_id()
            .map_err(|error| error.to_string())?
            .to_string();
        Ok(Self { id, corpus })
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        bounded(bytes)?;
        let artifact: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        artifact.validate()?;
        Ok(artifact)
    }

    pub fn to_json(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        bounded(&bytes)?;
        Ok(bytes)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        self.corpus
            .canonical_form()
            .map_err(|error| error.to_string())
    }

    pub fn validate(&self) -> Result<(), String> {
        self.corpus.validate()?;
        let id = ContentId::from_str(&self.id).map_err(|error| error.to_string())?;
        if id.to_string() != self.id {
            return Err("the CID must use canonical base32-lower presentation".into());
        }
        self.corpus
            .ensure_content_id(&id)
            .map_err(|error| error.to_string())
    }
}

impl Corpus {
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        bounded(bytes)?;
        let corpus = Self::from_canonical_form(bytes).map_err(|error| error.to_string())?;
        corpus.validate()?;
        Ok(corpus)
    }
}

fn bounded(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_ARTIFACT_BYTES {
        Err("corpus artifacts are limited to 8 MiB".into())
    } else {
        Ok(())
    }
}
