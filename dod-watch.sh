#!/usr/bin/env bash

# Helper script for developing.
# Makes it easy to launch all components you're currently working on and have
# them reload on change with the help of `watchexec`

# FG colors
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
RESET='\033[0m'

args=("$@")

declare -a components

for crate in ./crates/*; do
    if [[ -e "$crate/src/main.rs" ]]; then
        components+=("$(basename "$crate")")
    fi
done

if (("${#args[@]}" == 0)); then
    echo -e "${GREEN}[Help]${RESET} Pass components to run them using watchexec"
    echo -e "${GREEN}[Help]${RESET} Valid components: ${CYAN}${components[*]}${RESET}"
fi

for arg in "${args[@]}"; do
    # check if the arg is invalid. aka is a not a component
    if ! printf "%s\n" "${components[@]}" | grep -qx "$arg"; then
        echo -e "${RED}[Error]${RESET} Invalid component: $arg"
        echo -e "${GREEN}[Help]${RESET} Valid components: ${CYAN}${components[*]}${RESET}"
        exit 1
    fi
done

for component in "${args[@]}"; do
    watchexec -w crates -r --stop-signal SIGKILL -- cargo run --bin dod-shell-"$component" &
done

wait
