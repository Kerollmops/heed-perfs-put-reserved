use heed::byteorder::BigEndian;
use heed::{types::*, RwTxn};
use roaring_bitmap_codec::RoaringBitmapCodec;
use std::mem;
use std::{fs, iter, time::Instant};

mod roaring_bitmap_codec;

const TEN_GIBIBYTES: usize = 10 * 1024 * 1024 * 1024;

type BEU32 = U32<BigEndian>;

use clap::{Parser, ValueEnum};
use heed::{Database, EnvOpenOptions};
use roaring::RoaringBitmap;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_enum, default_value_t)]
    put_method: PutMethod,
}

/// Doc comment
#[derive(Clone, ValueEnum, Debug, Default)]
enum PutMethod {
    #[default]
    ClassicCodec,
    PutReserved,
    PutReservedUninit,
    PutReservedUninitIntoSlice,
}

fn main() -> anyhow::Result<()> {
    let Args { put_method } = Args::parse();
    let bitmap = generate_bitmap();

    let database_name = uuid::Uuid::new_v4().to_string();
    let database_path = format!("{database_name}.mdb");
    fs::create_dir_all(&database_path)?;
    let env = EnvOpenOptions::new()
        .map_size(TEN_GIBIBYTES)
        .open(database_path)?;

    let mut wtxn = env.write_txn()?;
    let db: Database<BEU32, RoaringBitmapCodec> = env.create_database(&mut wtxn, None)?;

    let before = Instant::now();
    for x in 0..100_000 {
        match put_method {
            PutMethod::ClassicCodec => put_in_db_codec(&mut wtxn, db, x, &bitmap)?,
            PutMethod::PutReserved => put_in_db_reserved(&mut wtxn, db, x, &bitmap)?,
            PutMethod::PutReservedUninit => put_in_db_reserved_uninit(&mut wtxn, db, x, &bitmap)?,
            PutMethod::PutReservedUninitIntoSlice => {
                put_in_db_reserved_uninit_into_slice(&mut wtxn, db, x, &bitmap)?
            }
        }
    }

    wtxn.commit()?;

    eprintln!("{:.02?}", before.elapsed());

    Ok(())
}

fn generate_bitmap() -> RoaringBitmap {
    let mut rng = fastrand::Rng::with_seed(42);
    iter::repeat_with(|| rng.u32(..)).take(10_000).collect()
}

#[inline(never)]
fn put_in_db_codec(
    wtxn: &mut RwTxn,
    db: Database<BEU32, RoaringBitmapCodec>,
    n: u32,
    bitmap: &RoaringBitmap,
) -> heed::Result<()> {
    db.put(wtxn, &n, bitmap)
}

#[inline(never)]
fn put_in_db_reserved(
    wtxn: &mut RwTxn,
    db: Database<BEU32, RoaringBitmapCodec>,
    n: u32,
    bitmap: &RoaringBitmap,
) -> heed::Result<()> {
    db.put_reserved(wtxn, &n, bitmap.serialized_size(), |space| {
        bitmap.serialize_into(space)
    })
}

#[inline(never)]
fn put_in_db_reserved_uninit(
    wtxn: &mut RwTxn,
    db: Database<BEU32, RoaringBitmapCodec>,
    n: u32,
    bitmap: &RoaringBitmap,
) -> heed::Result<()> {
    let uninit = db.put_reserved_uninit(wtxn, &n, bitmap.serialized_size())?;
    let slice: &mut [u8] = unsafe { mem::transmute(uninit) };
    bitmap.serialize_into(slice).map_err(Into::into)
}

#[inline(never)]
fn put_in_db_reserved_uninit_into_slice(
    wtxn: &mut RwTxn,
    db: Database<BEU32, RoaringBitmapCodec>,
    n: u32,
    bitmap: &RoaringBitmap,
) -> heed::Result<()> {
    let uninit = db.put_reserved_uninit(wtxn, &n, bitmap.serialized_size())?;
    let slice: &mut [u8] = unsafe { mem::transmute(uninit) };
    bitmap.serialize_into_slice(slice).map_err(Into::into)
}
