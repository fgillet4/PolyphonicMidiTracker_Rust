#!/bin/bash

# Exit on error
set -e

# Set up color outputs
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Building Guitar MIDI Tracker plugin...${NC}"

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Rust is not installed. Please install Rust from https://rustup.rs/${NC}"
    exit 1
fi

# Build the plugin in release mode
echo -e "${GREEN}Building plugin in release mode...${NC}"
cargo build --release

# Bundle the plugin using nih-plug's xtask
echo -e "${GREEN}Bundling plugin...${NC}"
cargo xtask bundle guitar_midi_tracker --release

echo -e "${GREEN}Build completed successfully!${NC}"
echo -e "Plugin bundle can be found in target/bundled/"