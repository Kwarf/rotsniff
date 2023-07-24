# rotsniff

rotsniff is a tool to catalog files and their hashes in order to detect corrupted or missing files.

It was inspired by [scorch](https://github.com/trapexit/scorch), and the [database](#database) format is similar, but do
not expect any kind of compatibility at this time.

```
Usage: rotsniff [OPTIONS] <COMMAND>

Commands:
  append  Add files not found in the database
  remove  Remove entries from the database that no longer exists
  update  Update entries in the database for files that have changed
  verify  Verify that all files in the database are intact, and that all files have entries in the database
  help    Print this message or the help of the given subcommand(s)

Options:
      --db <FILE>            Path to the database file [default: ./rotsniff.db]
  -v, --verbose              Make `command` more verbose. Actual behavior depends on the command
  -f, --fnfilter <FNFILTER>  Restrict commands to files which match regex
  -F, --negate-fnfilter      Negate the fnfilter regex match
  -h, --help                 Print help
  -V, --version              Print version
```

## Installing

You can install the latest tagged version directly from [crates.io](https://crates.io/crates/rotsniff) by running the
following command.

```
cargo install rotsniff
```

## Examples

```
% mkdir foo
% echo 'Hello, World!' > foo/hello

% rotsniff -v append foo
foo/hello: blake2b:94D8520FE182ADD62BEC85B531A17A779FCD39F23248CFABD18347B86CE9F8B73A0C151DD7CE171843DD8A14E5329DDE6B73149D26D6638E94EF4C634F3F1A7B

% rotsniff -v verify foo
MATCH: foo/hello

% echo 'Goodbye!' > foo/hello

% rotsniff -v verify foo
MODIFIED: foo/hello

% rotsniff -v update
UPDATED: foo/hello

% rm foo/hello
% touch foo/new

% rotsniff -v verify foo
FILE NOT FOUND: foo/hello
NOT FOUND IN DB: foo/new

% rotsniff -v append foo
foo/new: blake2b:786A02F742015903C6C6FD852552D272912F4740E15847618A86E217F71F5419D25E1031AFEE585313896444934EB04B903A685B1448B755D56F701AFE9BE2CE

% rotsniff -v verify foo
FILE NOT FOUND: foo/hello
MATCH: foo/new

% rotsniff -v remove
REMOVED: foo/hello

% rotsniff -v verify foo
MATCH: foo/new
```

## Database

The database is a simple CSV text file that is compressed with gzip, in order to be future proof and easily parsed by
other software if required.

```
% rotsniff -v append foo
foo/test: blake2b:7DFDB888AF71EAE0E6A6B751E8E3413D767EF4FA52A7993DAA9EF097F7AA3D949199C113CAA37C94F80CF3B22F7D9D6E4F5DEF4FF927830CFFE4857C34BE3D89
% zcat < rotsniff.db
foo/test,blake2b:7DFDB888AF71EAE0E6A6B751E8E3413D767EF4FA52A7993DAA9EF097F7AA3D949199C113CAA37C94F80CF3B22F7D9D6E4F5DEF4FF927830CFFE4857C34BE3D89
```

The format is currently `file,hash:digest`, but this may change to include more data in the future. The only supported
hash function for now is [BLAKE2b](https://en.wikipedia.org/wiki/BLAKE_(hash_function)#BLAKE2).