#!/bin/sh

# before_install: script for Travis on OSX


brew update >/dev/null

# Install OpenSSL.
# (incantations taken from https://github.com/sfackler/rust-openssl/issues/255)
brew install openssl
export OPENSSL_INCLUDE_DIR=`brew --prefix openssl`/include
export OPENSSL_LIB_DIR=`brew --prefix openssl`/lib
export DEP_OPENSSL_INCLUDE=`brew --prefix openssl`/include
