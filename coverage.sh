#! /bin/bash

cat <<- EOF
Generates coverage report
Need to install:
    cargo install rustfilt
    rustup component add llvm-tools-preview
    cargo publish
    cargo install cargo-binutils
    sudo apt install jq
EOF

OBJECTS=$( \
      for file in \
        $( \
          RUSTFLAGS="-Zinstrument-coverage" \
            cargo test --tests --no-run --message-format=json \
              | jq -r "select(.profile.test == true) | .filenames[]" \
              | grep -v dSYM - \
        ); \
      do \
        printf "%s %s " -object $file; \
      done \
    )


RUST_BACKTRACE=1 RUSTFLAGS="-Zinstrument-coverage"   LLVM_PROFILE_FILE="coverage/coverage-%m.profraw"   cargo test --tests
cargo profdata -- merge     -sparse coverage/coverage-*.profraw -o coverage/coverage.profdata
cargo cov -- report --use-color --ignore-filename-regex='/.cargo/registry' --ignore-filename-regex='/rustc/'  \
    --instr-profile=coverage/coverage.profdata $OBJECTS

# cargo cov -- show     --use-color --ignore-filename-regex='/.cargo/registry' --ignore-filename-regex='/rustc/'     --instr-profile=json5format.profdata     --object target/debug/deps/dstream-0cacdb0ba3784a41  --show-instantiations --show-line-counts-or-regions Xdemangler=rustfil
# cargo cov -- export     --use-color --ignore-filename-regex='/.cargo/registry' --ignore-filename-regex='/rustc/'     --instr-profile=json5format.profdata     --object target/debug/deps/dstream-0cacdb0ba3784a41 --summary-only | jq -r .data[0].totals.lines.percent