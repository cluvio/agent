#!/bin/bash

set -e

EXECUTABLE=$1
MACOS_CERTIFICATE=$2
MACOS_CERTIFICATE_PWD=$3
IDENTITY=$4
APPLE_DEV_ACCOUNT=$5
APPLE_DEV_PASSWORD=$6
APP_NAME=$7

if [ -z "$EXECUTABLE" -o -z "$MACOS_CERTIFICATE" -o -z "$MACOS_CERTIFICATE_PWD" -o -z "$IDENTITY" -o -z "$APPLE_DEV_ACCOUNT" -o -z "$APPLE_DEV_PASSWORD" -o -z "$APP_NAME" ]; then
  echo "Usage: $0 <executable> <macos_certificate> <macos_certificate_password> <apple_identity> <apple_dev_account> <apple_dev_account_name> <app_name>"
  exit 1
fi

# Import certificate into newly created keychain
KEYCHAIN_PASS=$(uuidgen)
echo $MACOS_CERTIFICATE | base64 --decode > certificate.p12
security create-keychain -p $KEYCHAIN_PASS build.keychain
security default-keychain -s build.keychain
security unlock-keychain -p $KEYCHAIN_PASS build.keychain
security import certificate.p12 -k build.keychain -P "$MACOS_CERTIFICATE_PWD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k $KEYCHAIN_PASS build.keychain
security find-identity

# Codesign the executable with hardened runtime (--options runtime)
/usr/bin/codesign --force --options runtime -s $IDENTITY $EXECUTABLE -v

mkdir image-bundle
cp $EXECUTABLE image-bundle

# Create disk image
hdiutil create -volname "$APP_NAME" -srcfolder image-bundle -ov -format UDZO "$APP_NAME".dmg

# Codesign the disk image
/usr/bin/codesign --force --options runtime -s $IDENTITY "$APP_NAME".dmg -v

# Trigger notarization of the disk image
xcrun altool --notarize-app --username "$APPLE_DEV_ACCOUNT" --password "$APPLE_DEV_PASSWORD" --file "$APP_NAME".dmg --wait

# Staple the disk image
xcrun stapler staple "$APP_NAME".dmg
