#!/bin/bash
set -e

REPO="ZenXene/agent-wiki-os"

echo "Detecting OS and Architecture..."
OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" = "Darwin" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        ASSET="agent-wiki-os-macos-amd64"
    elif [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then
        ASSET="agent-wiki-os-macos-arm64"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
elif [ "$OS" = "Linux" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        ASSET="agent-wiki-os-linux-amd64"
    else
        echo "Unsupported architecture: $ARCH"
        exit 1
    fi
else
    echo "Unsupported OS: $OS"
    exit 1
fi

# Fetch the latest release URL
echo "Fetching latest release information..."
LATEST_URL=$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep "browser_download_url.*$ASSET" | cut -d '"' -f 4)

if [ -z "$LATEST_URL" ]; then
    echo "Could not find a release for $ASSET"
    exit 1
fi

echo "Downloading $LATEST_URL..."
curl -sL "$LATEST_URL" -o agent-wiki-os
chmod +x agent-wiki-os

echo "Installing to /usr/local/bin..."
if [ -w "/usr/local/bin" ]; then
    mv agent-wiki-os /usr/local/bin/
else
    sudo mv agent-wiki-os /usr/local/bin/
fi

echo "Installation complete! Try running 'agent-wiki-os --help'"