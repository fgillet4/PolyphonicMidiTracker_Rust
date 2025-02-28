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

# Create a complete VST3 bundle for macOS
PLUGIN_NAME="GuitarMIDITracker"
VST3_BUNDLE_DIR="$HOME/Library/Audio/Plug-Ins/VST3/${PLUGIN_NAME}.vst3"
CONTENTS_DIR="${VST3_BUNDLE_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo -e "${GREEN}Creating VST3 bundle structure...${NC}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

# Create Info.plist
echo -e "${GREEN}Creating Info.plist...${NC}"
cat > "${CONTENTS_DIR}/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleExecutable</key>
    <string>${PLUGIN_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>com.francisbrain.${PLUGIN_NAME}</string>
    <key>CFBundleName</key>
    <string>${PLUGIN_NAME}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CSResourcesFileMapped</key>
    <true/>
</dict>
</plist>
EOF

# Create PkgInfo
echo -e "${GREEN}Creating PkgInfo...${NC}"
echo "BNDL????" > "${CONTENTS_DIR}/PkgInfo"

# Copy the compiled plugin
echo -e "${GREEN}Copying plugin to VST3 bundle...${NC}"
cp "target/release/libguitar_midi_tracker.dylib" "${MACOS_DIR}/${PLUGIN_NAME}" || {
    echo -e "${RED}Failed to copy plugin to VST3 bundle${NC}"
    exit 1
}

# Code sign the plugin binary and bundle
echo -e "${GREEN}Code signing the plugin...${NC}"
codesign -f -s - "${MACOS_DIR}/${PLUGIN_NAME}" || {
    echo -e "${RED}Failed to sign the plugin binary${NC}"
    exit 1
}
codesign -f -s - "${VST3_BUNDLE_DIR}" || {
    echo -e "${RED}Failed to sign the VST3 bundle${NC}"
    exit 1
}

echo -e "${GREEN}Build completed successfully!${NC}"
echo -e "Plugin is installed at: ${VST3_BUNDLE_DIR}"
echo -e "${YELLOW}Please restart Ableton Live and rescan plugins${NC}"