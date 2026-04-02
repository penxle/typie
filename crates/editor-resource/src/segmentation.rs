use icu_provider::buf::AsDeserializingBufferProvider;
use icu_provider_blob::BlobDataProvider;
use icu_segmenter::options::{SentenceBreakOptions, WordBreakOptions};
use icu_segmenter::{SentenceSegmenter, WordSegmenter};

use crate::error::ResourceError;
use crate::zstd::decompress_zstd;

pub struct TextSegmenters {
    pub word: WordSegmenter,
    pub sentence: SentenceSegmenter,
}

impl TextSegmenters {
    pub fn from_icu_data(data: &[u8]) -> Result<Self, ResourceError> {
        let data = decompress_zstd(data)?;

        let provider =
            BlobDataProvider::try_new_from_static_blob(Box::leak(data.into_boxed_slice()))
                .map_err(|e| ResourceError::IcuProvider(e.to_string()))?;
        let dp = provider.as_deserializing();

        Ok(Self {
            word: WordSegmenter::try_new_dictionary_unstable(&dp, WordBreakOptions::default())
                .map_err(|e| ResourceError::IcuSegmenter(e.to_string()))?,
            sentence: SentenceSegmenter::try_new_unstable(&dp, SentenceBreakOptions::default())
                .map_err(|e| ResourceError::IcuSegmenter(e.to_string()))?,
        })
    }
}

#[cfg(feature = "test-utils")]
impl TextSegmenters {
    pub fn new_test() -> Self {
        Self {
            word: WordSegmenter::try_new_auto(WordBreakOptions::default()).unwrap(),
            sentence: SentenceSegmenter::try_new(SentenceBreakOptions::default()).unwrap(),
        }
    }
}
