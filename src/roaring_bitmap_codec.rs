use roaring::RoaringBitmap;
use std::{borrow::Cow, error::Error};

pub struct RoaringBitmapCodec;

impl heed::BytesDecode<'_> for RoaringBitmapCodec {
    type DItem = RoaringBitmap;

    fn bytes_decode(bytes: &[u8]) -> Result<Self::DItem, Box<dyn Error + Send + Sync + 'static>> {
        Ok(RoaringBitmap::deserialize_unchecked_from(bytes)?)
    }
}

impl heed::BytesEncode<'_> for RoaringBitmapCodec {
    type EItem = RoaringBitmap;

    fn bytes_encode(
        item: &Self::EItem,
    ) -> Result<Cow<[u8]>, Box<dyn Error + Send + Sync + 'static>> {
        let mut bytes = Vec::with_capacity(item.serialized_size());
        item.serialize_into(&mut bytes)?;
        Ok(Cow::Owned(bytes))
    }
}
