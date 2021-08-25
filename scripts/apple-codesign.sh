#!/bin/bash

set -e

EXECUTABLE=$1
MACOS_CERTIFICATE=$2
MACOS_CERTIFICATE_PWD=$3
KEYCHAIN_PASS=$(uuidgen)

echo $MACOS_CERTIFICATE | base64 --decode > certificate.p12
security create-keychain -p $KEYCHAIN_PASS build.keychain
security default-keychain -s build.keychain
security unlock-keychain -p $KEYCHAIN_PASS build.keychain
security import certificate.p12 -k build.keychain -P $MACOS_CERTIFICATE_PWD -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k $KEYCHAIN_PASS build.keychain
security find-identity
IDENTITY=$(security find-identity)
/usr/bin/codesign --force -s $IDENTITY $EXECUTABLE -v
