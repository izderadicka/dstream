#! /bin/bash

cat <<- EOF
Generates coverage report
Need to install:
    cargo install rustfilt
    rustup component add llvm-tools-preview
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
# cargo cov -- report --use-color --ignore-filename-regex='/.cargo/registry' --ignore-filename-regex='/rustc/'  \
#     --instr-profile=coverage/coverage.profdata $OBJECTS

cargo cov -- show  --ignore-filename-regex='/.cargo/registry' --ignore-filename-regex='/rustc/'     \
--instr-profile=coverage/coverage.profdata $OBJECTS --show-instantiation-summary --show-branch-summary --show-region-summary \
--Xdemangler=rustfilt --format=html >coverage/report.html

COVERAGE=$(cargo cov -- export --ignore-filename-regex='/.cargo/registry' --ignore-filename-regex='/rustc/' \
--instr-profile=coverage/coverage.profdata  $OBJECTS --summary-only | jq -r .data[0].totals.lines.percent)

printf "::set-output name=coverage::%.*f\n" 2 $COVERAGE

