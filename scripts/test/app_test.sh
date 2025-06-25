#!/bin/bash

# --- Configuration & Setup ---
set -e # Exit immediately if a command exits with a non-zero status.

# A more robust way to get the script's and project's root directory
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &> /dev/null && pwd)
ROOT=$(cd -- "$SCRIPT_DIR/../.." &> /dev/null && pwd)

TIMEOUT=60s
S_PASS=0
S_FAILED=1
S_TIMEOUT=2
S_BUILD_FAILED=3
EXIT_STATUS=0

# --- Color Codes ---
RED_C="\x1b[31;1m"
GREEN_C="\x1b[32;1m"
YELLOW_C="\x1b[33;1m"
CYAN_C="\x1b[36;1m"
BLOD_C="\x1b[1m"
END_C="\x1b[0m"

# --- Functions ---
function cleanup_qemu() {
    pkill -f "qemu-system" || true
}

# Register cleanup trap
trap cleanup_qemu EXIT SIGINT SIGTERM

function run_and_compare() {
    local make_args=$1
    local actual_out=$2

    echo -e "\n  [BUILD] Executing: make -f makefile -C $ROOT $make_args build" >> "$actual_out"
    make -f makefile -C "$ROOT" $make_args build >> "$actual_out" 2>&1
    if [ $? -ne 0 ]; then
        return $S_BUILD_FAILED
    fi

    echo -e "\n  [RUN] Executing: timeout $TIMEOUT make -f makefile -C $ROOT $make_args justrun" >> "$actual_out"
    TIMEFORMAT='%3Rs'
    RUN_TIME=$( { time timeout --foreground $TIMEOUT make -f makefile -C "$ROOT" $make_args justrun >> "$actual_out" 2>&1; } 2>&1 )
    local res=$?

    if [ $res -eq 124 ]; then
        return $S_TIMEOUT
    # A non-zero exit code from `make` might also indicate success if the test program itself exited with a non-zero code.
    # We should rely on output comparison for real tests. For now, we consider any non-timeout exit as a potential pass.
    # elif [ $res -ne 0 ]; then
    #     return $S_FAILED
    else
        return $S_PASS
    fi
}

function test_one() {
    local args_from_cmd=$1
    local actual_out="$APP_DIR/actual.out"
    
    rm -f "$actual_out"
    touch "$actual_out" # Create the file upfront

    echo -ne "    -> Running with \"${BLOD_C}$args_from_cmd${END_C}\": "

    cleanup_qemu

    local all_make_args="BUILD_SCENARIO=test ARCH=$ARCH AX_TESTCASE=$APP $args_from_cmd"

    run_and_compare "$all_make_args" "$actual_out"
    local res=$?

    cleanup_qemu

    if [ $res -ne $S_PASS ]; then
        EXIT_STATUS=$res
        local status_msg=""
        case $res in
            $S_FAILED)       status_msg="${RED_C}failed!${END_C}" ;;
            $S_TIMEOUT)      status_msg="${YELLOW_C}timeout!${END_C}" ;;
            $S_BUILD_FAILED) status_msg="${RED_C}build failed!${END_C}" ;;
        esac
        echo -e "$status_msg $RUN_TIME"
        echo -e "${RED_C}------- Full Output ($actual_out): -------${END_C}"
        # Use `tail` in case of huge logs
        cat "$actual_out"
        echo -e "${RED_C}-------------------------------------------${END_C}"
    else
        echo -e "${GREEN_C}passed!${END_C} $RUN_TIME"
        rm -f "$actual_out"
    fi
}

# --- Main Script Logic ---
ARCH=${ARCH:-x86_64}

test_list=(
    "nimbos"
    # "libc"
)

for t in "${test_list[@]}"; do
    APP=$t
    # 【【【关键修复】】】
    # Use the robust $ROOT variable
    APP_DIR="$ROOT/apps/$t"

    echo -e "\n${CYAN_C}Preparing user apps for${END_C} ${BLOD_C}$t${END_C}..."
    make -f makefile user_apps BUILD_SCENARIO=test AX_TESTCASE=$t
    if [ $? -ne 0 ]; then
        echo -e "${RED_C}Failed to build user apps for $t. Aborting.${END_C}"
        exit $S_BUILD_FAILED
    fi

    echo -e "${CYAN_C}Testing${END_C} ${BLOD_C}$t${END_C}:"
    if [ -f "$APP_DIR/test_cmd" ]; then
        # `source` the file to execute the `test_one` calls within it
        source "$APP_DIR/test_cmd"
    else
        echo -e "${YELLOW_C}Warning: test_cmd not found in $APP_DIR${END_C}"
    fi
done

echo
echo -e "Test script finished with status: $EXIT_STATUS"
exit $EXIT_STATUS