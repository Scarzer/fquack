# fquack

`fquack` is a DuckDB loadable extension written in Rust that streams FASTQ reads into analytical workflows. It registers a single table function, `fquack(path)`, that returns one row per FASTQ record with columns for read metadata, nucleotide sequence, and ASCII-encoded quality scores.

This library is early development and my way of exploring Rust and expanding my understanding of DuckDB. **Use at your own risk (For now)**! 

## Features
- Uses DuckDB's virtual table API (`duckdb` crate with `vtab-loadable` feature) so queries can treat reads as relational data.
- Packages the compiled shared library into a `.duckdb_extension` artifact for easy loading from DuckDB shells or applications.

## Prerequisites
- Rust toolchain (1.74 or newer recommended).
- Python 3 with `venv` support.
- GNU Make.
- DuckDB (for running queries against the extension).

Clone the repository with submodules:

```sh
git clone --recurse-submodules git@github.com:Scarzer/fquack.git
cd fquack
```

## Build and Package
1. Configure the workspace (installs the DuckDB SQLLogic harness and records the host platform):
	```sh
	make configure
	```
2. Build a debug extension artifact:
	```sh
	make debug
	```
	The packaged binary is written to `build/debug/extension/fquack/fquack.duckdb_extension`.
3. For an optimized artifact, run:
	```sh
	make release
	```
	The release build is placed in `build/release/extension/fquack/`.

## Loading the Extension in DuckDB
Launch DuckDB with unsigned extensions enabled, load the local artifact, and query a FASTQ file.

```sh
duckdb -unsigned
```

```sql
LOAD './build/debug/extension/fquack/fquack.duckdb_extension';
```

Create a tiny sample FASTQ (optional) and query it:

```sh
cat <<'EOF' > example.fastq
@read1
ACGT
+
!!!!
@read2
TTGC
+
####
EOF
```

```sql
SELECT metadata, sequence, quality
FROM fquack('example.fastq');
```

Output columns:
- `metadata`: FASTQ identifier (text after the leading `@`).
- `sequence`: nucleotide string (A/C/G/T/N).
- `quality`: ASCII-encoded quality scores as stored in the file.

## Switching DuckDB Versions
To target a different DuckDB binary:

```sh
make clean_all
DUCKDB_TEST_VERSION=v1.3.2 make configure
make debug
make test_debug
```

Ensure that `TARGET_DUCKDB_VERSION` in `Makefile` matches the DuckDB crate versions in `Cargo.toml` to avoid ABI mismatches.

## Inspiration
This extension is based off the [rust extension template](https://github.com/duckdb/extension-template-rs) and draws heavily from the following projects:
- [fqkit](https://github.com/BioinfoToolbox/fqkit/tree/main)
- [pcap_reader_bfe](https://github.com/Overdrive83/pcap_reader_bfe/tree/main)
- [duckdb-file-tools](https://github.com/nicad/duckdb-file-tools/blob/main/src/lib.rs)
- [seq_io](https://docs.rs/seq_io/latest/seq_io/#example-fastq-parser)