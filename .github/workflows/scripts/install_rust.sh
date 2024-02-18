#!/usr/bin/env bash

#
# Copyright (c) 2024-present, Alessandro Gario
# All rights reserved.
#
# This source code is licensed in accordance with the terms specified in
# the LICENSE file found in the root directory of this source tree.
#

main() {
  which curl > /dev/null 2>&1
  if [[ $? != 0 ]] ; then
    printf "The 'curl' command is not available. Please install it and try again.\n"
    return 1
  fi

  curl \
    --proto '=https' \
    --tlsv1.2 \
    -sSf \
    https://sh.rustup.rs > "rustup.sh"

  chmod +x "rustup.sh"

  ./rustup.sh -y --quiet
  if [[ $? != 0 ]] ; then
    printf "Failed to install Rust.\n"
    return 1
  fi

  return 0
}

main $@
exit $?
