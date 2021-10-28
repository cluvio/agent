#!/bin/bash

set -e

executable="$1"

if [ -z "$executable" ]; then
    echo "usage: $0 <executable>"
    exit 1
fi

name="$(basename $executable)"

if [ -z "$MACOS_CERTIFICATE" ]; then
    echo 'Missing $MACOS_CERTIFICATE'
    exit 1
fi

if [ -z "$MACOS_CERTIFICATE_PWD" ]; then
    echo 'Missing $MACOS_CERTIFICATE_PWD'
    exit 1
fi

if [ -z "$MACOS_DEV_IDENTITY" ]; then
    echo 'Missing $MACOS_DEV_IDENTITY'
    exit 1
fi

# Import certificate into newly created keychain
keychain_pass=$(openssl rand -base64 32)
echo "$MACOS_CERTIFICATE" | base64 --decode > certificate.p12
security create-keychain -p "$keychain_pass" build.keychain
security default-keychain -s build.keychain
security unlock-keychain -p "$keychain_pass" build.keychain
security import certificate.p12 -k build.keychain -P "$MACOS_CERTIFICATE_PWD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$keychain_pass" build.keychain
security find-identity

# Codesign the executable with hardened runtime (--options runtime)
/usr/bin/codesign --force --options runtime -s "$MACOS_DEV_IDENTITY" "$executable" -v

# Create disk image
srcdir=$(mktemp -d -t tmp.XXXXXXXXXX)
cp "$executable" "$srcdir"
hdiutil create -volname "$name" -srcfolder "$srcdir" -ov -format UDZO "${executable}.dmg"

# Codesign the disk image
/usr/bin/codesign --force --options runtime -s "$MACOS_DEV_IDENTITY" "${executable}.dmg" -v

rm -rf "$srcdir"
