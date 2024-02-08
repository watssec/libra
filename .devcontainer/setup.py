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

    # instantiate the template
    content = []
    with open(path_in) as reader:
        for line in reader:
            line = line.rstrip()
            if line == "ARG UID":
                line = "{}={}".format(line, uid)
            elif line == "ARG GID":
                line = "{}={}".format(line, gid)
            content.append(line)

    with open(path_out, "w") as writer:
        for line in content:
            writer.write(line + os.linesep)

    # done
    return 0


if __name__ == "__main__":
    sys.exit(main())
