use criterion::{BatchSize, BenchmarkGroup, Criterion, criterion_group, criterion_main, measurement::WallTime};
use rawler::{
  decoders::RawDecodeParams,
  devtools::rawdb::{get_rawdb_cache, rawdb_ensure_file},
  rawsource::RawSource,
};
use std::{
  hint::black_box,
  path::{Path, PathBuf},
  time::Duration,
};

/// Remove the file from the OS page cache to simulate a first-time open.
#[cfg(unix)]
fn evict_page_cache(path: &Path) {
  use std::os::fd::AsRawFd;
  let file = std::fs::File::open(path).expect("Failed to open sample");
  unsafe { libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) };
}

#[cfg(not(unix))]
fn evict_page_cache(_path: &Path) {}

fn open_preview(path: &Path, lazy: bool) {
  let rawfile = if lazy { RawSource::new_lazy(path) } else { RawSource::new(path) }.expect("Failed to open sample");
  let decoder = rawler::get_decoder(&rawfile).expect("No decoder");
  let preview = decoder.preview_image(&rawfile, &RawDecodeParams::default()).expect("Preview failed");
  black_box(preview);
}

fn open_raw_image(path: &Path, lazy: bool) {
  let rawfile = if lazy { RawSource::new_lazy(path) } else { RawSource::new(path) }.expect("Failed to open sample");
  let decoder = rawler::get_decoder(&rawfile).expect("No decoder");
  let image = decoder.raw_image(&rawfile, &RawDecodeParams::default(), false).expect("Decoder failed");
  black_box(image);
}

fn bench_sample(group: &mut BenchmarkGroup<'_, WallTime>, name: &str, sample: &Path, f: fn(&Path, bool)) {
  for (mode, lazy) in [("eager", false), ("lazy", true)] {
    group.bench_function(format!("{name}/{mode}/warm"), |b| b.iter(|| f(sample, lazy)));
    group.bench_function(format!("{name}/{mode}/cold"), |b| {
      b.iter_batched(|| evict_page_cache(sample), |_| f(sample, lazy), BatchSize::PerIteration)
    });
  }
}

fn samples() -> Vec<(&'static str, PathBuf)> {
  [
    (
      "canon_eos_5dmk2_cr2",
      "Canon",
      "EOS 5D Mark II",
      "raw_modes/Canon EOS 5D Mark II_ISO_100_RAW.CR2",
    ),
    (
      "canon_eos_m50_cr3_raw",
      "Canon",
      "EOS M50",
      "raw_modes/Canon EOS M50_RAW_ISO_100_nocrop_nodual.CR3",
    ),
    (
      "canon_eos_m50_cr3_craw",
      "Canon",
      "EOS M50",
      "raw_modes/Canon EOS M50_CRAW_ISO_100_nocrop_nodual.CR3",
    ),
    ("nikon_d3s_nef", "Nikon", "D3S", "raw_modes/NIKON D3S_ISO_200_12bits_Lossless.NEF"),
  ]
  .into_iter()
  .filter_map(|(name, make, model, sample)| match rawdb_ensure_file(&get_rawdb_cache(), make, model, sample) {
    Ok(path) => Some((name, path)),
    Err(err) => {
      eprintln!("Warning: Bench failed: {:?}", err);
      None
    }
  })
  .collect()
}

fn preview(c: &mut Criterion) {
  let samples = samples();

  let mut group = c.benchmark_group("open-preview");
  group.sample_size(30).measurement_time(Duration::from_secs(5));
  for (name, path) in &samples {
    bench_sample(&mut group, name, path, open_preview);
  }
  group.finish();

  let mut group = c.benchmark_group("open-raw-image");
  group.sample_size(10).measurement_time(Duration::from_secs(10));
  for (name, path) in &samples {
    bench_sample(&mut group, name, path, open_raw_image);
  }
  group.finish();
}

criterion_group!(benches, preview);
criterion_main!(benches);
