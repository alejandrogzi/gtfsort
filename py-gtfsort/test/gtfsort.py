#!/usr/bin/env python3

import gtfsortpy
from typing import Tuple

NTHREADS = 4

def get_test_file() -> Tuple[str]:
    tmp = gtfsortpy.get_test_file()
    out = tmp + '.sorted'
    return (tmp, out)

def main():
    tmp, out = get_test_file()
    gtfsortpy.sort(tmp, out, NTHREADS)


if __name__ == '__main__':
    main()

