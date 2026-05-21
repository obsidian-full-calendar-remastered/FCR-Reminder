#!/bin/bash

# FCR Reminder Developer Check Script (Bash)
# Performs formatting, linting, and testing checks to ensure clean state code health.

set -e

# Define color codes
GREEN='\033[0;32m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "\n${CYAN}=== [1/3] Running Cargo Format check ===${NC}"
if ! cargo fmt --all -- --check; then
    echo -e "${RED}Format check failed! Please run 'cargo fmt --all' to fix formatting.${NC}"
    exit 1
fi
echo -e "${GREEN}Format check passed!${NC}"

echo -e "\n${CYAN}=== [2/3] Running Cargo Clippy lints ===${NC}"
if ! cargo clippy --all-targets --all-features -- -D warnings; then
    echo -e "${RED}Clippy lints failed! Please fix compiler warnings/errors.${NC}"
    exit 1
fi
echo -e "${GREEN}Clippy lints passed!${NC}"

echo -e "\n${CYAN}=== [3/3] Running Cargo Tests ===${NC}"
if ! cargo test --all; then
    echo -e "${RED}Tests failed! Please resolve failing unit tests.${NC}"
    exit 1
fi
echo -e "${GREEN}All tests passed successfully!${NC}"

echo -e "\n${GREEN}==================================================${NC}"
echo -e "${GREEN}  Success: Codebase is fully formatted, clean, and tested!${NC}"
echo -e "${GREEN}==================================================${NC}"
