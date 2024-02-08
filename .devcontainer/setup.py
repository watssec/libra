#!/usr/bin/env python3

import os
import pathlib
import sys


def main() -> int:
    # resolve paths
    path_base = pathlib.Path(__file__).parent.resolve()
    path_in = os.path.join(path_base, "Dockerfile.in")
    path_out = os.path.join(path_base, "Dockerfile")

    # collect information
    uid = os.getuid()
    gid = os.getgid()

    with open(path_in) as reader:

        with open(path_out, "w") as writer:


    return 0


if __name__ == "__main__":
    sys.exit(main())
