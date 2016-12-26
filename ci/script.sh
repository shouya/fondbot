# `script` phase: you usually build, test and generate docs in this phase

set -ex

. $(dirname $0)/utils.sh

run_test_suite() {
    case $TARGET in
        # configure emulation for transparent execution of foreign binaries
        aarch64-unknown-linux-gnu)
            export QEMU_LD_PREFIX=/usr/aarch64-linux-gnu
            ;;
        arm*-unknown-linux-gnueabihf)
            export QEMU_LD_PREFIX=/usr/arm-linux-gnueabihf
            ;;
        *)
            ;;
    esac

    if [ ! -z "$QEMU_LD_PREFIX" ]; then
        # Run tests on a single thread when using QEMU user emulation
        export RUST_TEST_THREADS=1
    fi

    cargo build --target $TARGET --verbose
#    cargo run --target $TARGET
#    cargo test --target $TARGET

    # sanity check the file type
    file target/$TARGET/debug/fondbot
}

main() {
    run_test_suite
}

main
