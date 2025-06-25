#!/bin/bash

TIMEOUT=60s
EXIT_STATUS=0
ROOT=$(realpath $(dirname $0))/../
AX_ROOT=$ROOT/.arceos
S_PASS=0
S_FAILED=1
S_TIMEOUT=2
S_BUILD_FAILED=3

RED_C="\x1b[31;1m"
GREEN_C="\x1b[32;1m"
YELLOW_C="\x1b[33;1m"
CYAN_C="\x1b[36;1m"
BLOD_C="\x1b[1m"
END_C="\x1b[0m"

# 【【【1. 定义清理函数】】】
function cleanup_qemu() {
    pkill -f qemu-system-x86_64 || true
    pkill -f qemu-system-aarch64 || true
    pkill -f qemu-system-riscv64 || true
    pkill -f qemu-system-loongarch64 || true
}

# 【【【2. 注册退出陷阱】】】
# 这保证了脚本无论如何退出，都会执行一次最终的清理
trap cleanup_qemu EXIT

if [ -z "$ARCH" ]; then
    ARCH=x86_64
fi
if [ "$ARCH" != "x86_64" ] && [ "$ARCH" != "riscv64" ] && [ "$ARCH" != "aarch64" ] && [ "$ARCH" != "loongarch64" ]; then
    echo "Unknown architecture: $ARCH"
    exit $S_FAILED
fi


function compare() {
    # local actual=$1
    # local expect=$2
    # if [ ! -f "$expect" ]; then
    #     MSG="expected output file \"${BLOD_C}$expect${END_C}\" not found!"
    #     return $S_FAILED
    # fi
    # IFS=''
    # while read -r line; do
    #     local matched=$(grep -m1 -a "$line" < "$actual")
    #     if [ -z "$matched" ]; then
    #         MSG="pattern \"${BLOD_C}$line${END_C}\" not matched!"
    #         unset IFS
    #         return $S_FAILED
    #     fi
    # done < "$expect"
    # unset IFS
    return $S_PASS
}

function run_and_compare() {
    local args=$1
    # local expect=$2
    local actual=$2

    # 【【【新增：将完整的 make 参数写入日志，方便调试】】】
    echo "Executing: make -C $ROOT $make_args build" > "$actual"

    make -C "$ROOT" AX_TESTCASE=$APP $args SCENARIO=test build > "$actual" 2>&1
    if [ $? -ne 0 ]; then
        return $S_BUILD_FAILED
    fi

    TIMEFORMAT='%3Rs'
    RUN_TIME=$( { time { timeout --foreground $TIMEOUT make -C "$ROOT" AX_TESTCASE=$APP $args SCENARIO=test justrun > "$actual" 2>&1; }; } 2>&1 )
    local res=$?
    if [ $res == 124 ]; then
        return $S_TIMEOUT
    elif [ $res -ne 0 ]; then
        return $S_FAILED
    else
        return $S_PASS
    fi

    # compare "$actual" "$expect"
    # if [ $? -ne 0 ]; then
    #     return $S_FAILED
    # else
    #     return $S_PASS
    # fi
}


function test_one() {
    # local args=$1
    # local expect="$APP_DIR/$2"
    local args_from_cmd=$1 # 从 test_cmd 文件传来的参数
    local actual="$APP_DIR/actual.out"
    local config_file=$(realpath --relative-to=$AX_ROOT "$ROOT/configs/$ARCH.toml")
    args="$args ARCH=$ARCH ACCEL=y EXTRA_CONFIG=$config_file"
    # 【【【最终的参数拼接点】】】
    # 在这里一次性地、清晰地组合所有需要的参数
    local all_make_args="AX_TESTCASE=$APP SCENARIO=test $args_from_cmd ARCH=$ARCH ACCEL=y EXTRA_CONFIG=$config_file"
    
    rm -f "$actual"

    # 【【【精髓所在：打印简洁版，执行完整版】】】
    # 打印给用户的，是 test_cmd 里定义的简洁版本
    echo -ne "    run with \"${BLOD_C}$args_from_cmd${END_C}\": "

    cleanup_qemu

    run_and_compare "$all_make_args" "$actual"
    local res=$?

    # 【【【4. 在每次测试后显式清理】】】
    cleanup_qemu

    # MSG=
    # run_and_compare "$args" "$expect" "$actual"
    # local res=$?

    if [ $res -ne $S_PASS ]; then
        EXIT_STATUS=$res
        if [ $res == $S_FAILED ]; then
            echo -e "${RED_C}failed!${END_C} $RUN_TIME"
        elif [ $res == $S_TIMEOUT ]; then
            echo -e "${YELLOW_C}timeout!${END_C} $RUN_TIME"
        elif [ $res == $S_BUILD_FAILED ]; then
            echo -e "${RED_C}build failed!${END_C}"
        fi
        if [ ! -z "$MSG" ]; then
            echo -e "        $MSG"
        fi
        echo -e "${RED_C}actual output${END_C}:"
        cat "$actual"
    else
        echo -e "${GREEN_C}passed!${END_C} $RUN_TIME"
        rm -f "$actual"
    fi
}

# TODO: add more testcases
test_list=(
    "nimbos"
    "libc"
)

for t in ${test_list[@]}; do
    APP=$t
    APP_DIR=$(realpath "$(pwd)/apps/$t")
    # make -C "$ROOT" user_apps SCENARIO=test AX_TESTCASE=$t
    echo -e "${CYAN_C}Testing${END_C} $t:"
    source "$APP_DIR/test_cmd"
done

echo -e "test script exited with: $EXIT_STATUS"
exit $EXIT_STATUS
