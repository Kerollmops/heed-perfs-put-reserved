use heed::{types::*, RwTxn};
use roaring_bitmap_codec::RoaringBitmapCodec;
use std::io::Write;
use std::mem::MaybeUninit;
use std::{fs, iter, time::Instant};

mod roaring_bitmap_codec;

const TEN_GIBIBYTES: usize = 10 * 1024 * 1024 * 1024;

type BEU32 = Bytes;

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
    PutReservedAlloc,
    PutReservedUninit,
    PutReservedUninitFillZeroes,
    PutReservedUninitIntoSlice,
}

fn main() -> anyhow::Result<()> {
    let Args { put_method } = Args::parse();
    let bitmap = generate_bitmap();

    let database_name = uuid::Uuid::new_v4().to_string();
    let database_path = format!("{database_name}.mdb");
    fs::create_dir_all(&database_path)?;
    let env = unsafe {
        EnvOpenOptions::new()
            .map_size(TEN_GIBIBYTES)
            .open(database_path)?
    };

    let mut wtxn = env.write_txn()?;
    let db: Database<BEU32, RoaringBitmapCodec> = env.create_database(&mut wtxn, None)?;

    let insert = Instant::now();
    for x in 0..100_000 {
        let bitmap = std::hint::black_box(&bitmap);
        let func = match put_method {
            PutMethod::ClassicCodec => put_in_db_codec,
            PutMethod::PutReserved => put_in_db_reserved,
            PutMethod::PutReservedAlloc => put_in_db_reserved_alloc,
            PutMethod::PutReservedUninit => put_in_db_reserved_uninit,
            PutMethod::PutReservedUninitFillZeroes => put_in_db_reserved_uninit_fill_zeroes,
            PutMethod::PutReservedUninitIntoSlice => put_in_db_reserved_uninit_into_slice,
        };
        func(&mut wtxn, db, x, bitmap)?;
    }
    let insert = insert.elapsed();

    let commit = Instant::now();
    wtxn.commit()?;
    let commit = commit.elapsed();

    let total = insert + commit;
    eprintln!("{total:>8.02?} [insert: {insert:>8.02?}, commit: {commit:>8.02?}]",);

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
    db.put(wtxn, &n.to_ne_bytes(), bitmap)
}

#[inline(never)]
fn put_in_db_reserved(
    wtxn: &mut RwTxn,
    db: Database<BEU32, RoaringBitmapCodec>,
    n: u32,
    bitmap: &RoaringBitmap,
) -> heed::Result<()> {
    db.put_reserved(wtxn, &n.to_ne_bytes(), bitmap.serialized_size(), |space| {
        bitmap.serialize_into(space)?;
        Ok(())
    })
}

#[inline(never)]
fn put_in_db_reserved_alloc(
    wtxn: &mut RwTxn,
    db: Database<BEU32, RoaringBitmapCodec>,
    n: u32,
    bitmap: &RoaringBitmap,
) -> heed::Result<()> {
    let size = bitmap.serialized_size();
    db.put_reserved(wtxn, &n.to_ne_bytes(), size, |space| {
        let mut bytes = Vec::with_capacity(size);
        bitmap.serialize_into(&mut bytes)?;
        space.write_all(&bytes)?;
        Ok(())
    })
}

#[inline(never)]
fn put_in_db_reserved_uninit(
    wtxn: &mut RwTxn,
    db: Database<BEU32, RoaringBitmapCodec>,
    n: u32,
    bitmap: &RoaringBitmap,
) -> heed::Result<()> {
    let size = bitmap.serialized_size();
    db.put_reserved(wtxn, &n.to_ne_bytes(), size, |space| {
        bitmap.serialize_into(UninitWriter(space.as_uninit_mut()))?;
        unsafe { space.assume_written(size) };
        Ok(())
    })
}

#[inline(never)]
fn put_in_db_reserved_uninit_fill_zeroes(
    wtxn: &mut RwTxn,
    db: Database<BEU32, RoaringBitmapCodec>,
    n: u32,
    bitmap: &RoaringBitmap,
) -> heed::Result<()> {
    let size = bitmap.serialized_size();
    db.put_reserved(wtxn, &n.to_ne_bytes(), size, |space| {
        space.fill_zeroes();
        bitmap.serialize_into(UninitWriter(space.as_uninit_mut()))?;
        Ok(())
    })
}

#[inline(never)]
fn put_in_db_reserved_uninit_into_slice(
    wtxn: &mut RwTxn,
    db: Database<BEU32, RoaringBitmapCodec>,
    n: u32,
    bitmap: &RoaringBitmap,
) -> heed::Result<()> {
    let size = bitmap.serialized_size();
    db.put_reserved(wtxn, &n.to_ne_bytes(), size, |space| {
        space.fill_zeroes();
        bitmap.serialize_into(space.written_mut())?;
        Ok(())
    })
}

struct UninitWriter<'a>(&'a mut [std::mem::MaybeUninit<u8>]);

impl std::io::Write for UninitWriter<'_> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_all(buf)?;
        Ok(buf.len())
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if buf.len() > self.0.len() {
            return Err(std::io::ErrorKind::WriteZero.into());
        }

        let buf_uninit = as_uninit_slice(buf);
        let (a, b) = std::mem::take(&mut self.0).split_at_mut(buf.len());
        a.copy_from_slice(buf_uninit);
        self.0 = b;

        Ok(())
    }

    #[inline(always)]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn as_uninit_slice<T>(slice: &[T]) -> &[MaybeUninit<T>] {
    // SAFETY: we can always cast `T` -> `MaybeUninit<T>` as it's a transparent wrapper
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), slice.len()) }
}
